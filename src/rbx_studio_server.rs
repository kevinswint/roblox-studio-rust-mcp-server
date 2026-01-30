use crate::error::Result;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use base64::Engine;
use color_eyre::eyre::{Error, OptionExt};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::oneshot::Receiver;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

pub const STUDIO_PLUGIN_PORT: u16 = 44755;
const LONG_POLL_DURATION: Duration = Duration::from_secs(15);

// Screenshot configuration
// Max 1920px to stay under API's 2000px limit for multi-image requests
const SCREENSHOT_MAX_DIMENSION: u32 = 1920;
const SCREENSHOT_JPEG_QUALITY: u8 = 85;
const SCREENSHOT_TIMEOUT_SECS: u64 = 10;
// Tool execution timeout - must be longer than Lua-side verification timeout (10s)
const TOOL_EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

// Script source for auto-installation
const MCP_INPUT_POLLER_SOURCE: &str = r#"-- Auto-installed by MCP Server for input simulation support
local HttpService = game:GetService("HttpService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")

-- Configuration
local MCP_URL = "http://localhost:44755/mcp/input"
local POLL_INTERVAL = 0.1  -- Poll every 100ms

-- Create RemoteEvent for server->client communication
local inputEvent = ReplicatedStorage:FindFirstChild("MCPInputCommand")
if not inputEvent then
    inputEvent = Instance.new("RemoteEvent")
    inputEvent.Name = "MCPInputCommand"
    inputEvent.Parent = ReplicatedStorage
end

-- Send command to all connected players
local function processCommand(command)
    print("[MCPPoller] Received:", command.command_type)
    for _, player in Players:GetPlayers() do
        inputEvent:FireClient(player, command)
    end
end

-- Main polling loop
local function pollLoop()
    print("[MCPPoller] Started - polling " .. MCP_URL)
    while true do
        local success, result = pcall(function()
            local response = HttpService:GetAsync(MCP_URL)
            return HttpService:JSONDecode(response)
        end)

        if success and result and result.commands then
            if #result.commands > 0 then
                print("[MCPPoller] Got", #result.commands, "commands!")
            end
            for _, command in ipairs(result.commands) do
                processCommand(command)
            end
        end

        task.wait(POLL_INTERVAL)
    end
end

-- Start polling in background
task.spawn(pollLoop)
print("[MCPPoller] Script loaded - waiting for playtest to start polling")
"#;

const MCP_INPUT_HANDLER_SOURCE: &str = r#"-- Auto-installed by MCP Server for input simulation support
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")

local player = Players.LocalPlayer

-- Key name to KeyCode mapping
local KEY_MAP = {
    A = Enum.KeyCode.A, B = Enum.KeyCode.B, C = Enum.KeyCode.C, D = Enum.KeyCode.D,
    E = Enum.KeyCode.E, F = Enum.KeyCode.F, G = Enum.KeyCode.G, H = Enum.KeyCode.H,
    I = Enum.KeyCode.I, J = Enum.KeyCode.J, K = Enum.KeyCode.K, L = Enum.KeyCode.L,
    M = Enum.KeyCode.M, N = Enum.KeyCode.N, O = Enum.KeyCode.O, P = Enum.KeyCode.P,
    Q = Enum.KeyCode.Q, R = Enum.KeyCode.R, S = Enum.KeyCode.S, T = Enum.KeyCode.T,
    U = Enum.KeyCode.U, V = Enum.KeyCode.V, W = Enum.KeyCode.W, X = Enum.KeyCode.X,
    Y = Enum.KeyCode.Y, Z = Enum.KeyCode.Z,
    Space = Enum.KeyCode.Space,
    Return = Enum.KeyCode.Return,
    Tab = Enum.KeyCode.Tab,
    Escape = Enum.KeyCode.Escape,
    Backspace = Enum.KeyCode.Backspace,
    LeftShift = Enum.KeyCode.LeftShift,
    RightShift = Enum.KeyCode.RightShift,
    LeftControl = Enum.KeyCode.LeftControl,
    RightControl = Enum.KeyCode.RightControl,
    LeftAlt = Enum.KeyCode.LeftAlt,
    RightAlt = Enum.KeyCode.RightAlt,
    Up = Enum.KeyCode.Up,
    Down = Enum.KeyCode.Down,
    Left = Enum.KeyCode.Left,
    Right = Enum.KeyCode.Right,
    One = Enum.KeyCode.One, Two = Enum.KeyCode.Two, Three = Enum.KeyCode.Three,
    Four = Enum.KeyCode.Four, Five = Enum.KeyCode.Five, Six = Enum.KeyCode.Six,
    Seven = Enum.KeyCode.Seven, Eight = Enum.KeyCode.Eight, Nine = Enum.KeyCode.Nine,
    Zero = Enum.KeyCode.Zero,
}

local MOUSE_MAP = {
    Left = Enum.UserInputType.MouseButton1,
    Right = Enum.UserInputType.MouseButton2,
    Middle = Enum.UserInputType.MouseButton3,
}

local function findGui(path)
    if not player.PlayerGui then return nil end
    local parts = string.split(path, ".")
    local current = player.PlayerGui
    if parts[1] == "PlayerGui" or parts[1] == "StarterGui" then
        table.remove(parts, 1)
    end
    for _, part in ipairs(parts) do
        current = current:FindFirstChild(part)
        if not current then return nil end
    end
    return current
end

local function getEvent(name)
    local e = ReplicatedStorage:FindFirstChild(name)
    if not e then
        e = Instance.new("BindableEvent")
        e.Name = name
        e.Parent = ReplicatedStorage
    end
    return e
end

local function handleKeyboard(data)
    local keyCode = KEY_MAP[data.key]
    if not keyCode then
        warn("[MCPInput] Unknown key:", data.key)
        return
    end
    local event = getEvent("MCPInputReceived")
    local info = {
        KeyCode = keyCode,
        UserInputType = Enum.UserInputType.Keyboard
    }
    if data.action == "tap" then
        info.UserInputState = Enum.UserInputState.Begin
        event:Fire(info)
        task.wait(0.05)
        info.UserInputState = Enum.UserInputState.End
        event:Fire(info)
    else
        info.UserInputState = data.action == "begin"
            and Enum.UserInputState.Begin
            or Enum.UserInputState.End
        event:Fire(info)
    end
    print("[MCPInput] Key:", data.key, data.action)
end

local function handleMouse(data)
    local button = MOUSE_MAP[data.key]
    if not button then
        warn("[MCPInput] Unknown mouse button:", data.key)
        return
    end
    local event = getEvent("MCPInputReceived")
    local info = {
        UserInputType = button,
        Position = Vector3.new(data.mouseX or 0, data.mouseY or 0, 0)
    }
    if data.action == "tap" then
        info.UserInputState = Enum.UserInputState.Begin
        event:Fire(info)
        task.wait(0.05)
        info.UserInputState = Enum.UserInputState.End
        event:Fire(info)
    else
        info.UserInputState = data.action == "begin"
            and Enum.UserInputState.Begin
            or Enum.UserInputState.End
        event:Fire(info)
    end
    print("[MCPInput] Mouse:", data.key, data.action)
end

local function handleGuiClick(data)
    local element = findGui(data.path)
    if not element then
        warn("[MCPInput] GUI not found:", data.path)
        return
    end
    local event = getEvent("MCPGuiClicked")
    event:Fire({
        element = element,
        path = data.path,
        absolutePosition = element.AbsolutePosition,
        absoluteSize = element.AbsoluteSize,
    })
    print("[MCPInput] GUI clicked:", data.path)
end

local inputCommand = ReplicatedStorage:WaitForChild("MCPInputCommand", 10)
if inputCommand then
    inputCommand.OnClientEvent:Connect(function(command)
        local data = command.data
        if command.command_type == "input" then
            if data.inputType == "keyboard" then
                handleKeyboard(data)
            elseif data.inputType == "mouse" then
                handleMouse(data)
            end
        elseif command.command_type == "gui_click" then
            handleGuiClick(data)
        end
    end)
    print("[MCPInput] Client handler ready!")
else
    warn("[MCPInput] Failed to find MCPInputCommand - is MCPInputPoller running?")
end
"#;

// Movement handler - translates MCPInputReceived events into actual character movement
const MCP_MOVEMENT_HANDLER_SOURCE: &str = r#"-- Auto-installed by MCP Server for character movement simulation
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")
local RunService = game:GetService("RunService")

local player = Players.LocalPlayer

-- Track which keys are "pressed" via MCP
local keysDown = {
    W = false, A = false, S = false, D = false, Space = false,
}

-- Get or create the MCP input event
local mcpInputEvent = ReplicatedStorage:FindFirstChild("MCPInputReceived")
if not mcpInputEvent then
    mcpInputEvent = Instance.new("BindableEvent")
    mcpInputEvent.Name = "MCPInputReceived"
    mcpInputEvent.Parent = ReplicatedStorage
end

-- Handle MCP input events
mcpInputEvent.Event:Connect(function(inputInfo)
    local keyCode = inputInfo.KeyCode
    local state = inputInfo.UserInputState

    local keyName = nil
    if keyCode == Enum.KeyCode.W then keyName = "W"
    elseif keyCode == Enum.KeyCode.A then keyName = "A"
    elseif keyCode == Enum.KeyCode.S then keyName = "S"
    elseif keyCode == Enum.KeyCode.D then keyName = "D"
    elseif keyCode == Enum.KeyCode.Space then keyName = "Space"
    end

    if keyName then
        keysDown[keyName] = (state == Enum.UserInputState.Begin)
    end
end)

-- Movement loop
RunService.Heartbeat:Connect(function(dt)
    local character = player.Character
    if not character then return end

    local humanoid = character:FindFirstChild("Humanoid")
    local rootPart = character:FindFirstChild("HumanoidRootPart")
    if not humanoid or not rootPart then return end

    local moveDir = Vector3.zero
    local camera = workspace.CurrentCamera

    if camera then
        local camCF = camera.CFrame
        local forward = camCF.LookVector * Vector3.new(1, 0, 1)
        local right = camCF.RightVector * Vector3.new(1, 0, 1)

        if forward.Magnitude > 0 then forward = forward.Unit end
        if right.Magnitude > 0 then right = right.Unit end

        if keysDown.W then moveDir = moveDir + forward end
        if keysDown.S then moveDir = moveDir - forward end
        if keysDown.D then moveDir = moveDir + right end
        if keysDown.A then moveDir = moveDir - right end
    end

    if moveDir.Magnitude > 0 then
        moveDir = moveDir.Unit
        humanoid:Move(moveDir, false)
    end

    if keysDown.Space and humanoid.FloorMaterial ~= Enum.Material.Air then
        humanoid:ChangeState(Enum.HumanoidStateType.Jumping)
        keysDown.Space = false
    end
end)

print("[MCPMovement] Handler ready - WASD and Space supported")
"#;

// Click support - makes click_gui trigger real button clicks
const MCP_CLICK_SUPPORT_SOURCE: &str = r#"-- Auto-installed by MCP Server for GUI click simulation
-- Use: local MCP = require(game.ReplicatedStorage.MCPClickSupport)
--      MCP.onClick(button, function() ... end)

local ReplicatedStorage = game:GetService("ReplicatedStorage")

local MCPClickSupport = {}

local mcpEvent = ReplicatedStorage:FindFirstChild("MCPGuiClicked")
if not mcpEvent then
    mcpEvent = Instance.new("BindableEvent")
    mcpEvent.Name = "MCPGuiClicked"
    mcpEvent.Parent = ReplicatedStorage
end

function MCPClickSupport.onClick(button, callback)
    button.MouseButton1Click:Connect(callback)
    mcpEvent.Event:Connect(function(data)
        if data.element == button then
            callback()
        end
    end)
end

print("[MCPClickSupport] Module loaded")

return MCPClickSupport
"#;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ToolArguments {
    args: ToolArgumentValues,
    id: Option<Uuid>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RunCommandResponse {
    response: String,
    id: Uuid,
}

/// Command for input simulation - queued by MCP tools, polled by game
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InputCommand {
    pub command_type: String,  // "keyboard", "mouse", "gui_click"
    pub data: serde_json::Value,
    pub id: Uuid,
    pub timestamp: u64,
}

/// Command for server-side code execution - queued by MCP, polled by game ServerScript
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerCodeCommand {
    pub id: Uuid,
    pub code: String,
    pub timestamp: u64,
}

/// Result from server-side code execution
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerCodeResult {
    pub id: Uuid,
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

// Timeout for waiting for server code execution result
const SERVER_CODE_TIMEOUT: Duration = Duration::from_secs(30);

pub struct AppState {
    process_queue: VecDeque<ToolArguments>,
    output_map: HashMap<Uuid, mpsc::UnboundedSender<Result<String>>>,
    waiter: watch::Receiver<()>,
    trigger: watch::Sender<()>,
    /// Queue of input commands for game to poll
    pub input_command_queue: VecDeque<InputCommand>,
    /// Queue of server code commands for game to poll
    pub server_code_queue: VecDeque<ServerCodeCommand>,
    /// Map of pending server code result channels (waiting for game to respond)
    pub server_code_results: HashMap<Uuid, mpsc::UnboundedSender<ServerCodeResult>>,
}
pub type PackedState = Arc<Mutex<AppState>>;

impl AppState {
    pub fn new() -> Self {
        let (trigger, waiter) = watch::channel(());
        Self {
            process_queue: VecDeque::new(),
            output_map: HashMap::new(),
            waiter,
            trigger,
            input_command_queue: VecDeque::new(),
            server_code_queue: VecDeque::new(),
            server_code_results: HashMap::new(),
        }
    }
}

impl ToolArguments {
    fn new(args: ToolArgumentValues) -> (Self, Uuid) {
        Self { args, id: None }.with_id()
    }
    fn with_id(self) -> (Self, Uuid) {
        let id = Uuid::new_v4();
        (
            Self {
                args: self.args,
                id: Some(id),
            },
            id,
        )
    }
}
#[derive(Clone)]
pub struct RBXStudioServer {
    state: PackedState,
    tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for RBXStudioServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "Roblox_Studio".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Roblox Studio MCP Server".to_string()),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "User run_command to query data from Roblox Studio place or to change it"
                    .to_string(),
            ),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunCode {
    #[schemars(description = "Code to run")]
    command: String,
}
#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct InsertModel {
    #[schemars(description = "Query to search for the model")]
    query: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct WriteScript {
    #[schemars(
        description = "Path to script in game hierarchy (e.g., 'ServerScriptService.GameManager')"
    )]
    path: String,
    #[schemars(description = "The Luau source code to write to the script")]
    source: String,
    #[schemars(
        description = "Type of script to create: 'Script', 'LocalScript', or 'ModuleScript'. Defaults to 'Script'. Only used when creating new scripts."
    )]
    script_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct CaptureScreenshot {
    // No parameters for v1 - just capture the Studio window
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct ReadOutput {
    #[schemars(
        description = "Filter output by level: 'all' (default), 'print', 'warn', or 'error'. Use 'error' to quickly find issues."
    )]
    filter: Option<String>,

    #[schemars(
        description = "Maximum number of lines to return (default: 1000, max: 10000). Returns most recent messages first."
    )]
    max_lines: Option<u32>,

    #[schemars(
        description = "Clear buffer after reading (default: true). Set to false to preserve history for subsequent reads."
    )]
    clear_after_read: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct GetStudioState {
    // No parameters - returns Studio state info
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct StartPlaytest {
    // No parameters - starts play mode with player character
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct StartSimulation {
    // No parameters - starts run mode without player
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct StopSimulation {
    // No parameters - stops play/run and returns to edit mode
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct StopPlaytest {
    // No parameters - stops playtest and returns to edit mode (alias for stop_simulation)
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct MoveCharacter {
    #[schemars(description = "Target X coordinate in world space")]
    x: f64,
    #[schemars(description = "Target Y coordinate in world space")]
    y: f64,
    #[schemars(description = "Target Z coordinate in world space")]
    z: f64,
    #[schemars(
        description = "If true, teleport instantly. If false, walk to position using Humanoid:MoveTo()"
    )]
    instant: Option<bool>,
    #[schemars(
        description = "Optional character name to move. If not specified, moves the first character found in workspace."
    )]
    character_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct SimulateInput {
    #[schemars(description = "Type of input: 'keyboard' or 'mouse'")]
    input_type: String,
    #[schemars(
        description = "For keyboard: key name (e.g., 'W', 'Space', 'E', 'LeftShift'). For mouse: 'Left', 'Right', 'Middle'"
    )]
    key: String,
    #[schemars(description = "Action type: 'begin' (key down), 'end' (key up), or 'tap' (quick press and release)")]
    action: String,
    #[schemars(description = "For mouse input: X position on screen")]
    mouse_x: Option<f64>,
    #[schemars(description = "For mouse input: Y position on screen")]
    mouse_y: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct ClickGui {
    #[schemars(
        description = "Path to the GUI element (e.g., 'PlayerGui.MainUI.PlayButton' or 'StarterGui.ScreenGui.Button')"
    )]
    path: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct RunServerCode {
    #[schemars(description = "Luau code to execute in the server context during playtest")]
    code: String,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct FireRemote {
    #[schemars(description = "Path to RemoteEvent (e.g., 'ReplicatedStorage.Remotes.PlayerAction')")]
    path: String,
    #[schemars(
        description = "Direction: 'ToServer' (from client), 'ToClient' (to specific player), 'ToAllClients'"
    )]
    direction: String,
    #[schemars(
        description = "JSON array of arguments to pass to the RemoteEvent (e.g., '[\"action\", {\"data\": 1}]')"
    )]
    args: Option<String>,
    #[schemars(description = "For 'ToClient' direction: player name to send to")]
    player_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct ValidateUI {
    #[schemars(description = "Optional path to ScreenGui to validate (e.g., 'StarterGui.MainUI'). If not specified, validates all ScreenGuis.")]
    path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct CreateResponsiveLayout {
    #[schemars(description = "Name for the ScreenGui (e.g., 'MainUI')")]
    name: String,
    #[schemars(description = "Array of container positions to create: 'TopLeft', 'TopRight', 'TopCenter', 'BottomLeft', 'BottomRight', 'BottomCenter', 'CenterLeft', 'CenterRight', 'Center'")]
    containers: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
struct PreviewLayout {
    #[schemars(description = "Target viewport width in pixels (e.g., 390 for iPhone 14)")]
    width: f64,
    #[schemars(description = "Target viewport height in pixels (e.g., 844 for iPhone 14)")]
    height: f64,
    #[schemars(description = "Optional path to specific ScreenGui. If not specified, previews all ScreenGuis.")]
    path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema, Clone)]
enum ToolArgumentValues {
    RunCode(RunCode),
    InsertModel(InsertModel),
    WriteScript(WriteScript),
    ReadOutput(ReadOutput),
    GetStudioState(GetStudioState),
    StartPlaytest(StartPlaytest),
    StartSimulation(StartSimulation),
    StopSimulation(StopSimulation),
    StopPlaytest(StopPlaytest),
    MoveCharacter(MoveCharacter),
    ValidateUI(ValidateUI),
    CreateResponsiveLayout(CreateResponsiveLayout),
    PreviewLayout(PreviewLayout),
    // Note: SimulateInput and ClickGui are handled directly by Rust (HTTP polling)
    // and don't go through the Luau plugin
}
#[tool_router]
impl RBXStudioServer {
    pub fn new(state: PackedState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Runs a command in Roblox Studio and returns the printed output. Can be used to both make changes and retrieve information"
    )]
    async fn run_code(
        &self,
        Parameters(args): Parameters<RunCode>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::RunCode(args))
            .await
    }

    #[tool(
        description = "Inserts a model from the Roblox marketplace into the workspace. Returns the inserted model name."
    )]
    async fn insert_model(
        &self,
        Parameters(args): Parameters<InsertModel>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::InsertModel(args))
            .await
    }

    #[tool(
        description = "Creates or updates a Script, LocalScript, or ModuleScript with the provided Luau source code. Uses ScriptEditorService to safely write script source."
    )]
    async fn write_script(
        &self,
        Parameters(args): Parameters<WriteScript>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::WriteScript(args))
            .await
    }

    #[tool(
        description = "Captures a screenshot of the Roblox Studio window and returns it as a JPEG image. Useful for visual debugging, verifying UI changes, or analyzing the workspace layout."
    )]
    async fn capture_screenshot(
        &self,
        Parameters(_args): Parameters<CaptureScreenshot>,
    ) -> Result<CallToolResult, ErrorData> {
        // Rust-only implementation - no plugin communication needed
        match Self::take_studio_screenshot().await {
            Ok(base64_data) => Ok(CallToolResult::success(vec![Content::image(
                base64_data,
                "image/jpeg",
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to capture screenshot: {}",
                e
            ))])),
        }
    }

    #[tool(
        description = "Reads captured output from Roblox Studio's Output window. Captures print(), warn(), and error() messages during both Edit and Play modes. Useful for debugging scripts by checking for errors or reviewing print statements. The buffer holds up to 10,000 messages with FIFO eviction. Will warn if messages were dropped due to buffer overflow."
    )]
    async fn read_output(
        &self,
        Parameters(args): Parameters<ReadOutput>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::ReadOutput(args))
            .await
    }

    #[tool(
        description = "Gets the current Studio mode (edit/play/run) to determine if workspace modifications are safe"
    )]
    async fn get_studio_state(
        &self,
        Parameters(_args): Parameters<GetStudioState>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::GetStudioState(GetStudioState {}))
            .await
    }

    #[tool(
        description = "Starts playtest mode with a player character. Use this to test gameplay with player controls."
    )]
    async fn start_playtest(
        &self,
        Parameters(_args): Parameters<StartPlaytest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::StartPlaytest(StartPlaytest {}))
            .await
    }

    #[tool(
        description = "Starts simulation mode (run) without a player character. Use this to test physics and scripts without player interaction."
    )]
    async fn start_simulation(
        &self,
        Parameters(_args): Parameters<StartSimulation>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::StartSimulation(StartSimulation {}))
            .await
    }

    #[tool(
        description = "Stops playtest or simulation mode and returns to edit mode."
    )]
    async fn stop_simulation(
        &self,
        Parameters(_args): Parameters<StopSimulation>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::StopSimulation(StopSimulation {}))
            .await
    }

    #[tool(
        description = "Stops playtest mode and returns to edit mode. Alias for stop_simulation."
    )]
    async fn stop_playtest(
        &self,
        Parameters(_args): Parameters<StopPlaytest>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::StopPlaytest(StopPlaytest {}))
            .await
    }

    #[tool(
        description = "Moves or teleports a character in the workspace. Works in simulation mode where the plugin has direct access to the game. Use instant=true for teleporting or instant=false to walk to the position."
    )]
    async fn move_character(
        &self,
        Parameters(args): Parameters<MoveCharacter>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::MoveCharacter(args))
            .await
    }

    /// Internal helper to run a tool and get the raw string result
    async fn run_tool_raw(&self, args: ToolArgumentValues) -> Result<String, String> {
        let (command, id) = ToolArguments::new(args);
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<String>>();
        let trigger = {
            let mut state = self.state.lock().await;
            state.process_queue.push_back(command);
            state.output_map.insert(id, tx);
            state.trigger.clone()
        };

        if trigger.send(()).is_err() {
            return Err("Failed to trigger command".to_string());
        }

        let result = match timeout(TOOL_EXECUTION_TIMEOUT, rx.recv()).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                let mut state = self.state.lock().await;
                state.output_map.remove_entry(&id);
                return Err("Channel closed".to_string());
            }
            Err(_) => {
                let mut state = self.state.lock().await;
                state.output_map.remove_entry(&id);
                return Err("Timeout".to_string());
            }
        };

        {
            let mut state = self.state.lock().await;
            state.output_map.remove_entry(&id);
        }

        result.map_err(|e| e.to_string())
    }

    /// Check if MCP input scripts are installed, and install them if not.
    /// Returns (scripts_were_installed, error_message_if_any)
    async fn ensure_input_scripts_installed(&self) -> (bool, Option<String>) {
        // Check if all MCP scripts exist
        let check_code = r#"
            local poller = game:GetService("ServerScriptService"):FindFirstChild("MCPInputPoller")
            local sps = game:GetService("StarterPlayer"):FindFirstChild("StarterPlayerScripts")
            local handler = sps and sps:FindFirstChild("MCPInputHandler")
            local movement = sps and sps:FindFirstChild("MCPMovementHandler")
            local clickSupport = game:GetService("ReplicatedStorage"):FindFirstChild("MCPClickSupport")
            return tostring(poller ~= nil) .. "," .. tostring(handler ~= nil) .. "," .. tostring(movement ~= nil) .. "," .. tostring(clickSupport ~= nil)
        "#;

        let scripts_status = match self.run_tool_raw(ToolArgumentValues::RunCode(RunCode {
            command: check_code.to_string(),
        })).await {
            Ok(status) => status,
            Err(e) => {
                return (false, Some(format!("Failed to check scripts: {}", e)));
            }
        };

        let parts: Vec<&str> = scripts_status.trim().split(',').collect();
        let poller_exists = parts.first().map_or(false, |s| s.contains("true"));
        let handler_exists = parts.get(1).map_or(false, |s| s.contains("true"));
        let movement_exists = parts.get(2).map_or(false, |s| s.contains("true"));
        let click_support_exists = parts.get(3).map_or(false, |s| s.contains("true"));

        if poller_exists && handler_exists && movement_exists && click_support_exists {
            return (false, None); // Scripts already installed
        }

        // Install missing scripts
        let mut installed = Vec::new();

        if !poller_exists {
            if self.run_tool_raw(ToolArgumentValues::WriteScript(WriteScript {
                path: "ServerScriptService.MCPInputPoller".to_string(),
                source: MCP_INPUT_POLLER_SOURCE.to_string(),
                script_type: Some("Script".to_string()),
            })).await.is_err() {
                return (false, Some("Failed to install MCPInputPoller script".to_string()));
            }
            installed.push("MCPInputPoller (ServerScriptService)");
        }

        if !handler_exists {
            if self.run_tool_raw(ToolArgumentValues::WriteScript(WriteScript {
                path: "StarterPlayer.StarterPlayerScripts.MCPInputHandler".to_string(),
                source: MCP_INPUT_HANDLER_SOURCE.to_string(),
                script_type: Some("LocalScript".to_string()),
            })).await.is_err() {
                return (false, Some("Failed to install MCPInputHandler script".to_string()));
            }
            installed.push("MCPInputHandler (StarterPlayerScripts)");
        }

        if !movement_exists {
            if self.run_tool_raw(ToolArgumentValues::WriteScript(WriteScript {
                path: "StarterPlayer.StarterPlayerScripts.MCPMovementHandler".to_string(),
                source: MCP_MOVEMENT_HANDLER_SOURCE.to_string(),
                script_type: Some("LocalScript".to_string()),
            })).await.is_err() {
                return (false, Some("Failed to install MCPMovementHandler script".to_string()));
            }
            installed.push("MCPMovementHandler (StarterPlayerScripts)");
        }

        if !click_support_exists {
            if self.run_tool_raw(ToolArgumentValues::WriteScript(WriteScript {
                path: "ReplicatedStorage.MCPClickSupport".to_string(),
                source: MCP_CLICK_SUPPORT_SOURCE.to_string(),
                script_type: Some("ModuleScript".to_string()),
            })).await.is_err() {
                return (false, Some("Failed to install MCPClickSupport module".to_string()));
            }
            installed.push("MCPClickSupport (ReplicatedStorage)");
        }

        (true, if installed.is_empty() { None } else { Some(installed.join(", ")) })
    }

    #[tool(
        description = "Simulates keyboard or mouse input during playtest. Required scripts (MCPInputPoller, MCPInputHandler) will be auto-installed if missing. Supports keyboard keys (W, A, S, D, Space, E, etc.) and mouse buttons (Left, Right, Middle)."
    )]
    async fn simulate_input(
        &self,
        Parameters(args): Parameters<SimulateInput>,
    ) -> Result<CallToolResult, ErrorData> {
        // Check and install scripts if needed
        let (scripts_installed, installed_names) = self.ensure_input_scripts_installed().await;

        let command = InputCommand {
            command_type: "input".to_string(),
            data: serde_json::json!({
                "inputType": args.input_type,
                "key": args.key,
                "action": args.action,
                "mouseX": args.mouse_x,
                "mouseY": args.mouse_y,
            }),
            id: Uuid::new_v4(),
            timestamp: current_timestamp_ms(),
        };

        // POST to the HTTP server to ensure command reaches the right instance
        let client = reqwest::Client::new();
        let result = client
            .post(format!("http://127.0.0.1:{STUDIO_PLUGIN_PORT}/mcp/input"))
            .json(&serde_json::json!({ "command": command }))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                let mut message = format!(
                    "Queued {} input: {} {} (id: {}).",
                    args.input_type, args.key, args.action, command.id
                );
                if scripts_installed {
                    if let Some(names) = installed_names {
                        message.push_str(&format!(
                            "\n\n✅ Auto-installed required scripts: {}.\nNote: You must restart playtest (stop and F5 again) for the scripts to take effect.",
                            names
                        ));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Ok(response) => {
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to queue input command: HTTP {} - Is Roblox Studio running?",
                    response.status()
                ))]))
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to queue input command: {} - Is the MCP server running?",
                    e
                ))]))
            }
        }
    }

    #[tool(
        description = "Simulates clicking a GUI element during playtest. Required scripts (MCPInputPoller, MCPInputHandler) will be auto-installed if missing. Provide the path to the GUI element (e.g., 'ScreenGui.PlayButton')."
    )]
    async fn click_gui(
        &self,
        Parameters(args): Parameters<ClickGui>,
    ) -> Result<CallToolResult, ErrorData> {
        // Check and install scripts if needed
        let (scripts_installed, installed_names) = self.ensure_input_scripts_installed().await;

        let command = InputCommand {
            command_type: "gui_click".to_string(),
            data: serde_json::json!({
                "path": args.path,
            }),
            id: Uuid::new_v4(),
            timestamp: current_timestamp_ms(),
        };

        // POST to the HTTP server to ensure command reaches the right instance
        let client = reqwest::Client::new();
        let result = client
            .post(format!("http://127.0.0.1:{STUDIO_PLUGIN_PORT}/mcp/input"))
            .json(&serde_json::json!({ "command": command }))
            .send()
            .await;

        match result {
            Ok(response) if response.status().is_success() => {
                let mut message = format!(
                    "Queued GUI click: {} (id: {}).",
                    args.path, command.id
                );
                if scripts_installed {
                    if let Some(names) = installed_names {
                        message.push_str(&format!(
                            "\n\n✅ Auto-installed required scripts: {}.\nNote: You must restart playtest (stop and F5 again) for the scripts to take effect.",
                            names
                        ));
                    }
                }
                Ok(CallToolResult::success(vec![Content::text(message)]))
            }
            Ok(response) => {
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to queue GUI click: HTTP {} - Is Roblox Studio running?",
                    response.status()
                ))]))
            }
            Err(e) => {
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to queue GUI click: {} - Is the MCP server running?",
                    e
                ))]))
            }
        }
    }

    #[tool(
        description = "Executes Luau code in the server context during playtest. Unlike run_code which executes in the plugin context, this runs in the actual game server where ServerScriptService scripts execute. Requires MCPServerCodeRunner script in ServerScriptService. Use this to: verify server-side state, test game logic, check _G values set by server scripts, or invoke server functions."
    )]
    async fn run_server_code(
        &self,
        Parameters(args): Parameters<RunServerCode>,
    ) -> Result<CallToolResult, ErrorData> {
        let command_id = Uuid::new_v4();
        let command = ServerCodeCommand {
            id: command_id,
            code: args.code.clone(),
            timestamp: current_timestamp_ms(),
        };

        // Create channel to receive result
        let (tx, mut rx) = mpsc::unbounded_channel::<ServerCodeResult>();

        // Queue the command and register for result
        {
            let mut state = self.state.lock().await;
            state.server_code_queue.push_back(command);
            state.server_code_results.insert(command_id, tx);
        }

        // Wait for result with timeout
        let result = match timeout(SERVER_CODE_TIMEOUT, rx.recv()).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                // Channel closed without response
                let mut state = self.state.lock().await;
                state.server_code_results.remove(&command_id);
                return Ok(CallToolResult::error(vec![Content::text(
                    "Server code execution channel closed unexpectedly. Is the MCPServerCodeRunner script running?",
                )]));
            }
            Err(_) => {
                // Timeout elapsed
                let mut state = self.state.lock().await;
                state.server_code_results.remove(&command_id);
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Server code execution timed out after {}s. Ensure:\n\
                    1. Studio is in playtest mode (F5)\n\
                    2. MCPServerCodeRunner script is in ServerScriptService\n\
                    3. The script is polling http://localhost:{}/mcp/server_code",
                    SERVER_CODE_TIMEOUT.as_secs(),
                    STUDIO_PLUGIN_PORT
                ))]));
            }
        };

        // Clean up
        {
            let mut state = self.state.lock().await;
            state.server_code_results.remove(&command_id);
        }

        // Return result
        if result.success {
            Ok(CallToolResult::success(vec![Content::text(
                result.result.unwrap_or_else(|| "nil".to_string()),
            )]))
        } else {
            Ok(CallToolResult::error(vec![Content::text(format!(
                "Server code error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ))]))
        }
    }

    /// Helper to run generated code on server and wait for result
    async fn run_generated_server_code(&self, code: String) -> Result<CallToolResult, ErrorData> {
        let command_id = Uuid::new_v4();
        let command = ServerCodeCommand {
            id: command_id,
            code,
            timestamp: current_timestamp_ms(),
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<ServerCodeResult>();

        {
            let mut state = self.state.lock().await;
            state.server_code_queue.push_back(command);
            state.server_code_results.insert(command_id, tx);
        }

        let result = match timeout(SERVER_CODE_TIMEOUT, rx.recv()).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                let mut state = self.state.lock().await;
                state.server_code_results.remove(&command_id);
                return Ok(CallToolResult::error(vec![Content::text(
                    "Server execution channel closed. Is MCPServerCodeRunner running?",
                )]));
            }
            Err(_) => {
                let mut state = self.state.lock().await;
                state.server_code_results.remove(&command_id);
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Timed out after {}s. Ensure playtest is running with MCPServerCodeRunner.",
                    SERVER_CODE_TIMEOUT.as_secs()
                ))]));
            }
        };

        {
            let mut state = self.state.lock().await;
            state.server_code_results.remove(&command_id);
        }

        if result.success {
            Ok(CallToolResult::success(vec![Content::text(
                result.result.unwrap_or_else(|| "nil".to_string()),
            )]))
        } else {
            Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            ))]))
        }
    }

    #[tool(
        description = "Fires a RemoteEvent to clients. Supports 'ToClient' (to specific player) and 'ToAllClients' directions. Note: 'ToServer' is not supported because MCP runs on the server and RemoteEvent.OnServerEvent cannot be manually triggered. Requires MCPServerCodeRunner script in ServerScriptService."
    )]
    async fn fire_remote(
        &self,
        Parameters(args): Parameters<FireRemote>,
    ) -> Result<CallToolResult, ErrorData> {
        let fire_code = match args.direction.as_str() {
            "ToServer" => {
                // ToServer doesn't work from server context because:
                // 1. FireServer() is client-only
                // 2. OnServerEvent can't be manually triggered (it's an RBXScriptSignal, not BindableEvent)
                // For testing server event handlers, use run_server_code to call the handler function directly
                return Ok(CallToolResult::error(vec![Content::text(
                    "ToServer direction not supported: MCP runs on the server, and RemoteEvent.OnServerEvent \
                    cannot be manually triggered. To test server event handlers, use run_server_code to call \
                    the handler function directly, or use simulate_input/click_gui to trigger client actions \
                    that fire the remote."
                )]));
            }
            "ToClient" => {
                let player_name = args.player_name.as_deref().unwrap_or("Unknown");
                match &args.args {
                    Some(json_args) => format!(
                        "local HttpService = game:GetService('HttpService')\n\
                        local Players = game:GetService('Players')\n\
                        local player = Players:FindFirstChild('{}')\n\
                        if not player then error('Player not found: {}') end\n\
                        local args = HttpService:JSONDecode('{}')\n\
                        remote:FireClient(player, table.unpack(args))\n\
                        return 'Fired to client: {}'",
                        player_name, player_name, json_args.replace('\'', "\\'"), player_name
                    ),
                    None => format!(
                        "local Players = game:GetService('Players')\n\
                        local player = Players:FindFirstChild('{}')\n\
                        if not player then error('Player not found: {}') end\n\
                        remote:FireClient(player)\n\
                        return 'Fired to client: {}'",
                        player_name, player_name, player_name
                    ),
                }
            }
            "ToAllClients" => {
                match &args.args {
                    Some(json_args) => format!(
                        "local HttpService = game:GetService('HttpService')\n\
                        local args = HttpService:JSONDecode('{}')\n\
                        remote:FireAllClients(table.unpack(args))\n\
                        return 'Fired to all clients'",
                        json_args.replace('\'', "\\'")
                    ),
                    None => "remote:FireAllClients()\nreturn 'Fired to all clients'".to_string(),
                }
            }
            _ => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Invalid direction. Use 'ToServer', 'ToClient', or 'ToAllClients'",
                )]));
            }
        };

        let code = format!(
            r#"local path = "{}"
local parts = string.split(path, ".")
local current = game
for _, part in ipairs(parts) do
    current = current:FindFirstChild(part)
    if not current then
        error("Remote not found: " .. path .. " (failed at: " .. part .. ")")
    end
end
local remote = current
if not remote:IsA("RemoteEvent") then
    error("Object at " .. path .. " is not a RemoteEvent, it's a " .. remote.ClassName)
end
{}"#,
            args.path, fire_code
        );

        self.run_generated_server_code(code).await
    }

    #[tool(
        description = "Validates UI elements for common layout issues. Checks for: overlapping elements, offscreen elements, pixel positioning (Offset without Scale), missing UISizeConstraint, and AnchorPoint/Position mismatches. Returns a JSON report of issues found."
    )]
    async fn validate_ui(
        &self,
        Parameters(args): Parameters<ValidateUI>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::ValidateUI(args))
            .await
    }

    #[tool(
        description = "Creates a responsive UI layout with best-practice container structure. Creates a ScreenGui with positioned containers that include UISizeConstraint and UIListLayout for proper responsive behavior."
    )]
    async fn create_responsive_layout(
        &self,
        Parameters(args): Parameters<CreateResponsiveLayout>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::CreateResponsiveLayout(args))
            .await
    }

    #[tool(
        description = "Calculates what UI layout would look like at a specific viewport size (e.g., mobile device). Returns JSON with element positions and sizes, identifying elements that would be offscreen or overlapping. Useful for checking mobile layouts without the Device Emulator."
    )]
    async fn preview_layout(
        &self,
        Parameters(args): Parameters<PreviewLayout>,
    ) -> Result<CallToolResult, ErrorData> {
        self.generic_tool_run(ToolArgumentValues::PreviewLayout(args))
            .await
    }

    async fn generic_tool_run(
        &self,
        args: ToolArgumentValues,
    ) -> Result<CallToolResult, ErrorData> {
        let (command, id) = ToolArguments::new(args);
        tracing::debug!("Running command: {:?}", command);
        let (tx, mut rx) = mpsc::unbounded_channel::<Result<String>>();
        let trigger = {
            let mut state = self.state.lock().await;
            state.process_queue.push_back(command);
            state.output_map.insert(id, tx);
            state.trigger.clone()
        };
        trigger
            .send(())
            .map_err(|e| ErrorData::internal_error(format!("Unable to trigger send {e}"), None))?;

        // Wait for response with timeout to prevent hanging indefinitely
        let result = match timeout(TOOL_EXECUTION_TIMEOUT, rx.recv()).await {
            Ok(Some(result)) => result,
            Ok(None) => {
                // Channel closed without response
                let mut state = self.state.lock().await;
                state.output_map.remove_entry(&id);
                return Err(ErrorData::internal_error(
                    "Plugin channel closed without response",
                    None,
                ));
            }
            Err(_) => {
                // Timeout elapsed
                let mut state = self.state.lock().await;
                state.output_map.remove_entry(&id);
                return Err(ErrorData::internal_error(
                    format!(
                        "Tool execution timed out after {}s. The plugin may be unresponsive or Studio is in an unexpected state.",
                        TOOL_EXECUTION_TIMEOUT.as_secs()
                    ),
                    None,
                ));
            }
        };

        {
            let mut state = self.state.lock().await;
            state.output_map.remove_entry(&id);
        }
        tracing::debug!("Sending to MCP: {result:?}");
        match result {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(err) => Ok(CallToolResult::error(vec![Content::text(err.to_string())])),
        }
    }

    /// Process an image: resize to fit within max dimensions and encode as JPEG base64
    fn process_screenshot(img: image::DynamicImage) -> Result<String, Error> {
        // Resize to max dimensions while maintaining aspect ratio
        let resized = img.resize(
            SCREENSHOT_MAX_DIMENSION,
            SCREENSHOT_MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        );

        // Encode as JPEG
        let mut buffer = Vec::new();
        let rgb_image = resized.to_rgb8();
        let encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, SCREENSHOT_JPEG_QUALITY);
        rgb_image.write_with_encoder(encoder)?;

        // Encode to base64
        Ok(base64::engine::general_purpose::STANDARD.encode(&buffer))
    }

    #[cfg(target_os = "macos")]
    async fn take_studio_screenshot() -> Result<String, Error> {
        use std::fs;
        use std::io::Write;
        use tokio::process::Command;

        // Create temp files
        let temp_screenshot =
            std::env::temp_dir().join(format!("roblox_studio_{}.png", Uuid::new_v4()));
        let temp_swift =
            std::env::temp_dir().join(format!("get_window_{}.swift", Uuid::new_v4()));

        // Swift script to get window ID without requiring accessibility permissions
        let swift_script = r#"
import Cocoa
import CoreGraphics

let windowList = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []

for window in windowList {
    if let ownerName = window[kCGWindowOwnerName as String] as? String,
       ownerName.contains("Roblox"),
       let windowName = window[kCGWindowName as String] as? String,
       windowName.contains("Roblox Studio"),
       let windowNumber = window[kCGWindowNumber as String] as? Int {
        print(windowNumber)
        exit(0)
    }
}
exit(1)
"#;

        // Write Swift script to temp file
        let mut swift_file = fs::File::create(&temp_swift)?;
        swift_file.write_all(swift_script.as_bytes())?;
        drop(swift_file);

        // Get window ID for Roblox Studio using Swift (with timeout)
        let window_id_result = tokio::time::timeout(
            Duration::from_secs(SCREENSHOT_TIMEOUT_SECS),
            Command::new("swift").arg(&temp_swift).output(),
        )
        .await
        .map_err(|_| Error::msg("Timed out while finding Roblox Studio window"))?;

        let window_id_output = window_id_result?;

        // Clean up Swift temp file
        let _ = fs::remove_file(&temp_swift);

        if !window_id_output.status.success() {
            return Err(Error::msg(
                "Roblox Studio window not found. Please ensure Roblox Studio is open.",
            ));
        }

        let window_id_str = String::from_utf8_lossy(&window_id_output.stdout);
        let window_id = window_id_str
            .trim()
            .parse::<i32>()
            .map_err(|_| Error::msg("Failed to parse window ID"))?;

        // Capture the window (with timeout)
        let capture_result = tokio::time::timeout(
            Duration::from_secs(SCREENSHOT_TIMEOUT_SECS),
            Command::new("screencapture")
                .arg("-l")
                .arg(window_id.to_string())
                .arg("-o") // Disable window shadow
                .arg("-x") // No sound
                .arg(&temp_screenshot)
                .status(),
        )
        .await
        .map_err(|_| Error::msg("Timed out while capturing screenshot"))?;

        let capture = capture_result?;

        if !capture.success() {
            return Err(Error::msg(
                "Failed to capture screenshot. On macOS, ensure Screen Recording permission is granted in System Settings > Privacy & Security > Screen Recording.",
            ));
        }

        // Load image
        let img = image::open(&temp_screenshot)?;

        // Clean up screenshot temp file
        let _ = fs::remove_file(&temp_screenshot);

        // Process and encode
        Self::process_screenshot(img)
    }

    #[cfg(target_os = "windows")]
    async fn take_studio_screenshot() -> Result<String, Error> {
        use std::fs;
        use tokio::process::Command;

        // Create temp file for screenshot
        let temp_path =
            std::env::temp_dir().join(format!("roblox_studio_{}.png", Uuid::new_v4()));

        // PowerShell script to capture Roblox Studio window
        // Includes proper Win32 API type definitions for GetWindowRect
        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# Define Win32 API for GetWindowRect
Add-Type @"
using System;
using System.Runtime.InteropServices;

public struct RECT {{
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
}}

public class Win32 {{
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
}}
"@

$process = Get-Process | Where-Object {{ $_.MainWindowTitle -like "*Roblox Studio*" }} | Select-Object -First 1
if ($null -eq $process) {{
    Write-Error "No Roblox Studio window found"
    exit 1
}}

$handle = $process.MainWindowHandle
$rect = New-Object RECT
[Win32]::GetWindowRect($handle, [ref]$rect) | Out-Null

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)

$bitmap.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png)
"#,
            temp_path.display()
        );

        let capture_result = tokio::time::timeout(
            Duration::from_secs(SCREENSHOT_TIMEOUT_SECS),
            Command::new("powershell")
                .arg("-Command")
                .arg(&ps_script)
                .status(),
        )
        .await
        .map_err(|_| Error::msg("Timed out while capturing screenshot"))?;

        let capture = capture_result?;

        if !capture.success() {
            return Err(Error::msg(
                "Failed to capture screenshot. Is Roblox Studio running?",
            ));
        }

        // Load image
        let img = image::open(&temp_path)?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_path);

        // Process and encode
        Self::process_screenshot(img)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    async fn take_studio_screenshot() -> Result<String, Error> {
        Err(Error::msg(
            "Screenshot capture is only supported on macOS and Windows",
        ))
    }
}

pub async fn request_handler(State(state): State<PackedState>) -> Result<impl IntoResponse> {
    let timeout = tokio::time::timeout(LONG_POLL_DURATION, async {
        loop {
            let mut waiter = {
                let mut state = state.lock().await;
                if let Some(task) = state.process_queue.pop_front() {
                    return Ok::<ToolArguments, Error>(task);
                }
                state.waiter.clone()
            };
            waiter.changed().await?
        }
    })
    .await;
    match timeout {
        Ok(result) => Ok(Json(result?).into_response()),
        _ => Ok((StatusCode::LOCKED, String::new()).into_response()),
    }
}

pub async fn response_handler(
    State(state): State<PackedState>,
    Json(payload): Json<RunCommandResponse>,
) -> Result<impl IntoResponse> {
    tracing::debug!("Received reply from studio {payload:?}");
    let mut state = state.lock().await;
    let tx = state
        .output_map
        .remove(&payload.id)
        .ok_or_eyre("Unknown ID")?;
    Ok(tx.send(Ok(payload.response))?)
}

pub async fn proxy_handler(
    State(state): State<PackedState>,
    Json(command): Json<ToolArguments>,
) -> Result<impl IntoResponse> {
    let id = command.id.ok_or_eyre("Got proxy command with no id")?;
    tracing::debug!("Received request to proxy {command:?}");
    let (tx, mut rx) = mpsc::unbounded_channel();
    {
        let mut state = state.lock().await;
        state.process_queue.push_back(command);
        state.output_map.insert(id, tx);
    }
    let response = rx.recv().await.ok_or_eyre("Couldn't receive response")??;
    {
        let mut state = state.lock().await;
        state.output_map.remove_entry(&id);
    }
    tracing::debug!("Sending back to dud: {response:?}");
    Ok(Json(RunCommandResponse { response, id }))
}

pub async fn dud_proxy_loop(state: PackedState, exit: Receiver<()>) {
    let client = reqwest::Client::new();

    let mut waiter = { state.lock().await.waiter.clone() };
    while exit.is_empty() {
        let entry = { state.lock().await.process_queue.pop_front() };
        if let Some(entry) = entry {
            let res = client
                .post(format!("http://127.0.0.1:{STUDIO_PLUGIN_PORT}/proxy"))
                .json(&entry)
                .send()
                .await;
            if let Ok(res) = res {
                let tx = {
                    state
                        .lock()
                        .await
                        .output_map
                        .remove(&entry.id.unwrap())
                        .unwrap()
                };
                let res = res
                    .json::<RunCommandResponse>()
                    .await
                    .map(|r| r.response)
                    .map_err(Into::into);
                tx.send(res).unwrap();
            } else {
                tracing::error!("Failed to proxy: {res:?}");
            };
        } else {
            waiter.changed().await.unwrap();
        }
    }
}

/// Response for input polling endpoint
#[derive(Debug, Serialize)]
pub struct InputPollResponse {
    pub commands: Vec<InputCommand>,
    pub count: usize,
}

/// Request to add an input command (used by proxy instances)
#[derive(Debug, Deserialize)]
pub struct InputCommandRequest {
    pub command: InputCommand,
}

/// Handler for GET /mcp/input - Game polls this to get pending input commands
pub async fn get_input_commands_handler(
    State(state): State<PackedState>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    let commands: Vec<InputCommand> = state.input_command_queue.drain(..).collect();
    let count = commands.len();
    Json(InputPollResponse { commands, count })
}

/// Handler for POST /mcp/input - MCP tools post commands here (supports proxy mode)
pub async fn post_input_command_handler(
    State(state): State<PackedState>,
    Json(request): Json<InputCommandRequest>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    state.input_command_queue.push_back(request.command);
    (StatusCode::OK, "OK")
}

/// Response for server code polling endpoint
#[derive(Debug, Serialize)]
pub struct ServerCodePollResponse {
    pub commands: Vec<ServerCodeCommand>,
    pub count: usize,
}

/// Handler for GET /mcp/server_code - Game ServerScript polls this to get pending code
pub async fn get_server_code_handler(
    State(state): State<PackedState>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    let commands: Vec<ServerCodeCommand> = state.server_code_queue.drain(..).collect();
    let count = commands.len();
    Json(ServerCodePollResponse { commands, count })
}

/// Handler for POST /mcp/server_code - Game ServerScript posts execution results here
pub async fn post_server_code_result_handler(
    State(state): State<PackedState>,
    Json(result): Json<ServerCodeResult>,
) -> impl IntoResponse {
    let mut state = state.lock().await;
    if let Some(tx) = state.server_code_results.remove(&result.id) {
        if tx.send(result).is_err() {
            tracing::warn!("Failed to send server code result - receiver dropped");
        }
        (StatusCode::OK, "OK")
    } else {
        tracing::warn!("Received server code result for unknown id: {}", result.id);
        (StatusCode::NOT_FOUND, "Unknown command ID")
    }
}

/// Helper to get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

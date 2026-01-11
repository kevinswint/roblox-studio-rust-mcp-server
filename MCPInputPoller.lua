--[[
	MCP Input Simulation Scripts

	Input simulation requires scripts in your game because:
	- HTTP requests can only be made from ServerScripts
	- Input execution must happen on the client
	- Game systems need to listen to MCPInputReceived events

	INSTALLATION:
	1. Enable HttpService: Game Settings > Security > Allow HTTP Requests
	2. Copy MCPInputPoller to ServerScriptService
	3. Copy MCPInputHandler to StarterPlayerScripts
	4. (Optional) Copy MCPMovementController for WASD movement
	5. (Optional) Add ability integration to your ability scripts

	HOW IT WORKS:
	1. MCP tools (simulate_input, click_gui) queue commands to /mcp/input endpoint
	2. MCPInputPoller (server) polls the endpoint and sends commands to clients
	3. MCPInputHandler (client) receives commands and fires MCPInputReceived event
	4. Your game scripts listen to MCPInputReceived to respond to simulated input
]]

--============================================================================
-- SCRIPT 1: MCPInputPoller (ServerScript in ServerScriptService)
--============================================================================
--[[
local HttpService = game:GetService("HttpService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")

local MCP_URL = "http://localhost:44755/mcp/input"
local POLL_INTERVAL = 0.1

local inputEvent = ReplicatedStorage:FindFirstChild("MCPInputCommand")
if not inputEvent then
	inputEvent = Instance.new("RemoteEvent")
	inputEvent.Name = "MCPInputCommand"
	inputEvent.Parent = ReplicatedStorage
end

local function processCommand(command)
	print("[MCPPoller] Received:", command.command_type)
	for _, player in Players:GetPlayers() do
		inputEvent:FireClient(player, command)
	end
end

local function pollLoop()
	print("[MCPPoller] Server started - polling " .. MCP_URL)
	while true do
		local success, result = pcall(function()
			local response = HttpService:GetAsync(MCP_URL)
			return HttpService:JSONDecode(response)
		end)
		if success and result and result.commands then
			for _, command in ipairs(result.commands) do
				processCommand(command)
			end
		end
		task.wait(POLL_INTERVAL)
	end
end

task.spawn(pollLoop)
]]

--============================================================================
-- SCRIPT 2: MCPInputHandler (LocalScript in StarterPlayerScripts)
--============================================================================
--[[
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local Players = game:GetService("Players")

local player = Players.LocalPlayer

local KEY_MAP = {
	A = Enum.KeyCode.A, B = Enum.KeyCode.B, C = Enum.KeyCode.C, D = Enum.KeyCode.D,
	E = Enum.KeyCode.E, F = Enum.KeyCode.F, G = Enum.KeyCode.G, H = Enum.KeyCode.H,
	I = Enum.KeyCode.I, J = Enum.KeyCode.J, K = Enum.KeyCode.K, L = Enum.KeyCode.L,
	M = Enum.KeyCode.M, N = Enum.KeyCode.N, O = Enum.KeyCode.O, P = Enum.KeyCode.P,
	Q = Enum.KeyCode.Q, R = Enum.KeyCode.R, S = Enum.KeyCode.S, T = Enum.KeyCode.T,
	U = Enum.KeyCode.U, V = Enum.KeyCode.V, W = Enum.KeyCode.W, X = Enum.KeyCode.X,
	Y = Enum.KeyCode.Y, Z = Enum.KeyCode.Z,
	Space = Enum.KeyCode.Space, Return = Enum.KeyCode.Return,
	LeftShift = Enum.KeyCode.LeftShift, LeftControl = Enum.KeyCode.LeftControl,
	Up = Enum.KeyCode.Up, Down = Enum.KeyCode.Down, Left = Enum.KeyCode.Left, Right = Enum.KeyCode.Right,
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
	if not keyCode then return end
	local event = getEvent("MCPInputReceived")
	local info = { KeyCode = keyCode, UserInputType = Enum.UserInputType.Keyboard }
	if data.action == "tap" then
		info.UserInputState = Enum.UserInputState.Begin
		event:Fire(info)
		task.wait(0.05)
		info.UserInputState = Enum.UserInputState.End
		event:Fire(info)
	else
		info.UserInputState = data.action == "begin" and Enum.UserInputState.Begin or Enum.UserInputState.End
		event:Fire(info)
	end
	print("[MCPInput] Key:", data.key, data.action)
end

local function handleMouse(data)
	local button = MOUSE_MAP[data.key]
	if not button then return end
	local event = getEvent("MCPInputReceived")
	local info = { UserInputType = button, Position = Vector3.new(data.mouseX or 0, data.mouseY or 0, 0) }
	if data.action == "tap" then
		info.UserInputState = Enum.UserInputState.Begin
		event:Fire(info)
		task.wait(0.05)
		info.UserInputState = Enum.UserInputState.End
		event:Fire(info)
	else
		info.UserInputState = data.action == "begin" and Enum.UserInputState.Begin or Enum.UserInputState.End
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
	print("[MCPInput] GUI:", data.path)
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
end
]]

--============================================================================
-- SCRIPT 3: MCPMovementController (LocalScript in StarterPlayerScripts)
-- Enables WASD movement and jumping via MCP input simulation
--============================================================================
--[[
local Players = game:GetService("Players")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local RunService = game:GetService("RunService")

local Player = Players.LocalPlayer

-- Track which movement keys are pressed
local MovementKeys = {
	[Enum.KeyCode.W] = false,
	[Enum.KeyCode.A] = false,
	[Enum.KeyCode.S] = false,
	[Enum.KeyCode.D] = false,
	[Enum.KeyCode.Space] = false,
}

-- Get or create MCPInputReceived event
local MCPInputReceived = ReplicatedStorage:FindFirstChild("MCPInputReceived")
if not MCPInputReceived then
	MCPInputReceived = Instance.new("BindableEvent")
	MCPInputReceived.Name = "MCPInputReceived"
	MCPInputReceived.Parent = ReplicatedStorage
end

-- Handle MCP input
MCPInputReceived.Event:Connect(function(inputData)
	local keyCode = inputData.KeyCode

	if MovementKeys[keyCode] ~= nil then
		if inputData.UserInputState == Enum.UserInputState.Begin then
			MovementKeys[keyCode] = true

			-- Handle jump immediately
			if keyCode == Enum.KeyCode.Space then
				local character = Player.Character
				if character then
					local humanoid = character:FindFirstChild("Humanoid")
					if humanoid then
						humanoid:ChangeState(Enum.HumanoidStateType.Jumping)
					end
				end
			end
		elseif inputData.UserInputState == Enum.UserInputState.End then
			MovementKeys[keyCode] = false
		end
	end
end)

-- Calculate movement direction from pressed keys
local function getMovementDirection()
	local direction = Vector3.zero

	if MovementKeys[Enum.KeyCode.W] then
		direction = direction + Vector3.new(0, 0, -1)
	end
	if MovementKeys[Enum.KeyCode.S] then
		direction = direction + Vector3.new(0, 0, 1)
	end
	if MovementKeys[Enum.KeyCode.A] then
		direction = direction + Vector3.new(-1, 0, 0)
	end
	if MovementKeys[Enum.KeyCode.D] then
		direction = direction + Vector3.new(1, 0, 0)
	end

	if direction.Magnitude > 0 then
		direction = direction.Unit
	end

	return direction
end

-- Apply movement each frame
RunService.Heartbeat:Connect(function()
	local character = Player.Character
	if not character then return end

	local humanoid = character:FindFirstChild("Humanoid")
	local rootPart = character:FindFirstChild("HumanoidRootPart")
	if not humanoid or not rootPart then return end

	local moveDir = getMovementDirection()

	if moveDir.Magnitude > 0 then
		-- Convert local direction to world direction based on camera
		local camera = workspace.CurrentCamera
		if camera then
			local cameraDirection = camera.CFrame:VectorToWorldSpace(moveDir)
			cameraDirection = Vector3.new(cameraDirection.X, 0, cameraDirection.Z)
			if cameraDirection.Magnitude > 0 then
				humanoid:Move(cameraDirection.Unit, false)
			end
		else
			humanoid:Move(moveDir, false)
		end
	end
end)

print("[MCPMovementController] WASD + Space enabled via MCP input")
]]

--============================================================================
-- SCRIPT 4: Ability Integration Example
-- Add this pattern to your existing ability scripts to support MCP input
--============================================================================
--[[
-- Example: Add to bottom of your AbilityController or similar script

local ReplicatedStorage = game:GetService("ReplicatedStorage")

-- Your existing ability key mappings (adjust to match your game)
local AbilityKeys = {
	[Enum.KeyCode.Q] = "Ability1",
	[Enum.KeyCode.E] = "Ability2",
	[Enum.KeyCode.R] = "Ability3",
	[Enum.KeyCode.F] = "Ability4",
}

-- Your existing ability handlers (these should already exist in your code)
local AbilityHandlers = {
	Ability1 = function() --[[ your ability code ]] end,
	Ability2 = function() --[[ your ability code ]] end,
	Ability3 = function() --[[ your ability code ]] end,
	Ability4 = function() --[[ your ability code ]] end,
}

-- MCP Input Integration - Add this section
local MCPInputReceived = ReplicatedStorage:FindFirstChild("MCPInputReceived")
if not MCPInputReceived then
	MCPInputReceived = Instance.new("BindableEvent")
	MCPInputReceived.Name = "MCPInputReceived"
	MCPInputReceived.Parent = ReplicatedStorage
end

MCPInputReceived.Event:Connect(function(inputData)
	if inputData.UserInputState == Enum.UserInputState.Begin then
		local abilityName = AbilityKeys[inputData.KeyCode]
		if abilityName then
			-- Add cooldown check here if your game uses cooldowns
			local handler = AbilityHandlers[abilityName]
			if handler then
				print("[AbilityController] MCP triggered:", abilityName)
				handler()
			end
		end
	end
end)

print("[AbilityController] MCP input integration enabled!")
]]

--============================================================================
-- USAGE EXAMPLES
--============================================================================
--[[
After installing these scripts, you can use MCP tools to control your game:

-- Dismiss a welcome screen
click_gui({ path = "ScreenGui.WelcomeFrame.PlayButton" })

-- Move forward for 2 seconds
simulate_input({ input_type = "keyboard", key = "W", action = "begin" })
-- wait 2 seconds...
simulate_input({ input_type = "keyboard", key = "W", action = "end" })

-- Tap an ability key
simulate_input({ input_type = "keyboard", key = "E", action = "tap" })

-- Jump
simulate_input({ input_type = "keyboard", key = "Space", action = "tap" })

-- Strafe right while moving forward
simulate_input({ input_type = "keyboard", key = "W", action = "begin" })
simulate_input({ input_type = "keyboard", key = "D", action = "begin" })
-- wait...
simulate_input({ input_type = "keyboard", key = "W", action = "end" })
simulate_input({ input_type = "keyboard", key = "D", action = "end" })
]]

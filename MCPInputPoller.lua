--[[
	MCPInputPoller - Polls the MCP server for input commands during playtest

	INSTALLATION:
	1. Copy this script to StarterPlayerScripts in your game
	2. Enable HttpService in Game Settings > Security > Allow HTTP Requests
	3. The MCP server must be running on localhost:44755

	This script polls the MCP server's /mcp/input endpoint and executes
	input commands (keyboard, mouse, GUI clicks) sent via Claude.

	USAGE:
	After installing, use Claude with these MCP tools:
	- simulate_input(input_type="keyboard", key="E", action="tap")
	- simulate_input(input_type="mouse", key="Left", action="tap", mouse_x=500, mouse_y=300)
	- click_gui(path="ScreenGui.PlayButton")
]]

local HttpService = game:GetService("HttpService")
local UserInputService = game:GetService("UserInputService")
local Players = game:GetService("Players")

local MCP_URL = "http://localhost:44755/mcp/input"
local POLL_INTERVAL = 0.1 -- Poll every 100ms

-- Key name to Enum.KeyCode mapping
local KEY_MAP = {
	-- Letters
	A = Enum.KeyCode.A, B = Enum.KeyCode.B, C = Enum.KeyCode.C, D = Enum.KeyCode.D,
	E = Enum.KeyCode.E, F = Enum.KeyCode.F, G = Enum.KeyCode.G, H = Enum.KeyCode.H,
	I = Enum.KeyCode.I, J = Enum.KeyCode.J, K = Enum.KeyCode.K, L = Enum.KeyCode.L,
	M = Enum.KeyCode.M, N = Enum.KeyCode.N, O = Enum.KeyCode.O, P = Enum.KeyCode.P,
	Q = Enum.KeyCode.Q, R = Enum.KeyCode.R, S = Enum.KeyCode.S, T = Enum.KeyCode.T,
	U = Enum.KeyCode.U, V = Enum.KeyCode.V, W = Enum.KeyCode.W, X = Enum.KeyCode.X,
	Y = Enum.KeyCode.Y, Z = Enum.KeyCode.Z,
	-- Numbers
	One = Enum.KeyCode.One, Two = Enum.KeyCode.Two, Three = Enum.KeyCode.Three,
	Four = Enum.KeyCode.Four, Five = Enum.KeyCode.Five, Six = Enum.KeyCode.Six,
	Seven = Enum.KeyCode.Seven, Eight = Enum.KeyCode.Eight, Nine = Enum.KeyCode.Nine,
	Zero = Enum.KeyCode.Zero,
	-- Special
	Space = Enum.KeyCode.Space, Return = Enum.KeyCode.Return, Tab = Enum.KeyCode.Tab,
	Escape = Enum.KeyCode.Escape, Backspace = Enum.KeyCode.Backspace,
	-- Modifiers
	LeftShift = Enum.KeyCode.LeftShift, RightShift = Enum.KeyCode.RightShift,
	LeftControl = Enum.KeyCode.LeftControl, RightControl = Enum.KeyCode.RightControl,
	LeftAlt = Enum.KeyCode.LeftAlt, RightAlt = Enum.KeyCode.RightAlt,
	-- Arrows
	Up = Enum.KeyCode.Up, Down = Enum.KeyCode.Down, Left = Enum.KeyCode.Left, Right = Enum.KeyCode.Right,
	-- Function keys
	F1 = Enum.KeyCode.F1, F2 = Enum.KeyCode.F2, F3 = Enum.KeyCode.F3, F4 = Enum.KeyCode.F4,
	F5 = Enum.KeyCode.F5, F6 = Enum.KeyCode.F6, F7 = Enum.KeyCode.F7, F8 = Enum.KeyCode.F8,
	F9 = Enum.KeyCode.F9, F10 = Enum.KeyCode.F10, F11 = Enum.KeyCode.F11, F12 = Enum.KeyCode.F12,
}

local MOUSE_BUTTON_MAP = {
	Left = Enum.UserInputType.MouseButton1,
	Right = Enum.UserInputType.MouseButton2,
	Middle = Enum.UserInputType.MouseButton3,
}

-- Find GUI element by path
local function findGuiElement(path: string)
	local player = Players.LocalPlayer
	if not player then return nil end

	local parts = string.split(path, ".")
	local current: Instance? = nil

	-- Start from PlayerGui or StarterGui reference
	local firstPart = parts[1]
	if firstPart == "PlayerGui" or firstPart == "ScreenGui" then
		current = player:FindFirstChild("PlayerGui")
		table.remove(parts, 1)
	elseif firstPart == "StarterGui" then
		-- For StarterGui paths, look in PlayerGui
		current = player:FindFirstChild("PlayerGui")
		table.remove(parts, 1)
	else
		-- Assume it's in PlayerGui
		current = player:FindFirstChild("PlayerGui")
	end

	if not current then return nil end

	for _, part in ipairs(parts) do
		current = current:FindFirstChild(part)
		if not current then return nil end
	end

	return current
end

-- Execute a keyboard input command
local function executeKeyboardInput(data)
	local keyCode = KEY_MAP[data.key]
	if not keyCode then
		warn("[MCPInputPoller] Unknown key:", data.key)
		return
	end

	local action = data.action

	-- Create synthetic InputObject-like data for custom handlers
	local inputData = {
		KeyCode = keyCode,
		UserInputType = Enum.UserInputType.Keyboard,
		UserInputState = action == "begin" and Enum.UserInputState.Begin or Enum.UserInputState.End,
	}

	-- Fire a BindableEvent that game code can listen to
	local mcpEvent = game:GetService("ReplicatedStorage"):FindFirstChild("MCPInputReceived")
	if not mcpEvent then
		mcpEvent = Instance.new("BindableEvent")
		mcpEvent.Name = "MCPInputReceived"
		mcpEvent.Parent = game:GetService("ReplicatedStorage")
	end

	if action == "tap" then
		-- For tap, fire begin then end
		inputData.UserInputState = Enum.UserInputState.Begin
		mcpEvent:Fire(inputData)
		task.wait(0.05)
		inputData.UserInputState = Enum.UserInputState.End
		mcpEvent:Fire(inputData)
	else
		mcpEvent:Fire(inputData)
	end

	print("[MCPInputPoller] Keyboard:", data.key, action)
end

-- Execute a mouse input command
local function executeMouseInput(data)
	local button = MOUSE_BUTTON_MAP[data.key]
	if not button then
		warn("[MCPInputPoller] Unknown mouse button:", data.key)
		return
	end

	local action = data.action
	local position = Vector2.new(data.mouseX or 0, data.mouseY or 0)

	local inputData = {
		UserInputType = button,
		UserInputState = action == "begin" and Enum.UserInputState.Begin or Enum.UserInputState.End,
		Position = Vector3.new(position.X, position.Y, 0),
	}

	local mcpEvent = game:GetService("ReplicatedStorage"):FindFirstChild("MCPInputReceived")
	if not mcpEvent then
		mcpEvent = Instance.new("BindableEvent")
		mcpEvent.Name = "MCPInputReceived"
		mcpEvent.Parent = game:GetService("ReplicatedStorage")
	end

	if action == "tap" then
		inputData.UserInputState = Enum.UserInputState.Begin
		mcpEvent:Fire(inputData)
		task.wait(0.05)
		inputData.UserInputState = Enum.UserInputState.End
		mcpEvent:Fire(inputData)
	else
		mcpEvent:Fire(inputData)
	end

	print("[MCPInputPoller] Mouse:", data.key, action, "at", position)
end

-- Execute a GUI click command
local function executeGuiClick(data)
	local element = findGuiElement(data.path)
	if not element then
		warn("[MCPInputPoller] GUI element not found:", data.path)
		return
	end

	-- Try to activate the element
	if element:IsA("GuiButton") then
		-- Fire the Activated signal by calling the internal method
		-- Note: This is a workaround since we can't directly fire Activated
		local success, err = pcall(function()
			-- Try mouse enter/leave to trigger hover effects
			if element.MouseEnter then
				element.MouseEnter:Fire()
			end
		end)

		-- Fire a BindableEvent for the click
		local mcpEvent = game:GetService("ReplicatedStorage"):FindFirstChild("MCPGuiClicked")
		if not mcpEvent then
			mcpEvent = Instance.new("BindableEvent")
			mcpEvent.Name = "MCPGuiClicked"
			mcpEvent.Parent = game:GetService("ReplicatedStorage")
		end

		mcpEvent:Fire({
			element = element,
			path = data.path,
			absolutePosition = element.AbsolutePosition,
			absoluteSize = element.AbsoluteSize,
		})

		print("[MCPInputPoller] GUI Click:", data.path)
	else
		warn("[MCPInputPoller] Element is not a GuiButton:", data.path, element.ClassName)
	end
end

-- Process a single command
local function processCommand(command)
	local commandType = command.command_type
	local data = command.data

	if commandType == "input" then
		if data.inputType == "keyboard" then
			executeKeyboardInput(data)
		elseif data.inputType == "mouse" then
			executeMouseInput(data)
		end
	elseif commandType == "gui_click" then
		executeGuiClick(data)
	else
		warn("[MCPInputPoller] Unknown command type:", commandType)
	end
end

-- Main polling loop
local function pollLoop()
	while true do
		local success, result = pcall(function()
			local response = HttpService:GetAsync(MCP_URL)
			return HttpService:JSONDecode(response)
		end)

		if success and result and result.commands then
			for _, command in ipairs(result.commands) do
				processCommand(command)
			end
		elseif not success then
			-- Silently fail - MCP server might not be running
			-- Uncomment for debugging:
			-- warn("[MCPInputPoller] Poll failed:", result)
		end

		task.wait(POLL_INTERVAL)
	end
end

-- Start polling when player is ready
local player = Players.LocalPlayer
if player then
	print("[MCPInputPoller] Started - polling", MCP_URL)
	pollLoop()
end

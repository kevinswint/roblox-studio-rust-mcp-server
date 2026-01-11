--[[
	MCP Input Simulation Scripts

	Input simulation requires TWO scripts because:
	- HTTP requests can only be made from ServerScripts
	- Input execution must happen on the client

	INSTALLATION:
	1. Enable HttpService: Game Settings > Security > Allow HTTP Requests
	2. Copy MCPInputPoller to ServerScriptService
	3. Copy MCPInputHandler to StarterPlayerScripts
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
	print("[MCPPoller] VERIFIED RECEIVED:", command.command_type)
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

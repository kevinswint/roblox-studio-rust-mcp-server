--!strict
-- SimulateInput tool: Fire simulated input events via BindableEvent
-- Games can listen to these events to respond to simulated input for testing

local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local ReplicatedStorage = game:GetService("ReplicatedStorage")
local RunService = game:GetService("RunService")

-- Valid key names for keyboard input (maps to Enum.KeyCode)
local VALID_KEYS = {
	-- Letters
	A = true, B = true, C = true, D = true, E = true, F = true, G = true, H = true,
	I = true, J = true, K = true, L = true, M = true, N = true, O = true, P = true,
	Q = true, R = true, S = true, T = true, U = true, V = true, W = true, X = true,
	Y = true, Z = true,
	-- Numbers
	One = true, Two = true, Three = true, Four = true, Five = true,
	Six = true, Seven = true, Eight = true, Nine = true, Zero = true,
	-- Special keys
	Space = true, Return = true, Tab = true, Escape = true, Backspace = true,
	LeftShift = true, RightShift = true, LeftControl = true, RightControl = true,
	LeftAlt = true, RightAlt = true,
	-- Arrow keys
	Up = true, Down = true, Left = true, Right = true,
	-- Function keys
	F1 = true, F2 = true, F3 = true, F4 = true, F5 = true, F6 = true,
	F7 = true, F8 = true, F9 = true, F10 = true, F11 = true, F12 = true,
}

-- Valid mouse buttons
local VALID_MOUSE_BUTTONS = {
	Left = "MouseButton1",
	Right = "MouseButton2",
	Middle = "MouseButton3",
}

-- Valid actions
local VALID_ACTIONS = {
	begin = true,
	["end"] = true,
	tap = true,
}

-- Get or create the MCP input event in ReplicatedStorage
local function getOrCreateInputEvent(): BindableEvent
	local event = ReplicatedStorage:FindFirstChild("MCPInputEvent")
	if not event then
		event = Instance.new("BindableEvent")
		event.Name = "MCPInputEvent"
		event.Parent = ReplicatedStorage
	end
	return event :: BindableEvent
end

local function simulateInput(args: Types.SimulateInputArgs): string
	-- Note: During playtest, RunService:IsRunning() returns false from plugin context
	-- but we can still fire BindableEvents. We'll try and let it fail naturally.

	local inputType = string.lower(args.input_type)
	local action = string.lower(args.action)

	-- Validate action
	if not VALID_ACTIONS[action] then
		error(`Invalid action '{args.action}'. Must be 'begin', 'end', or 'tap'.`)
	end

	local inputEvent = getOrCreateInputEvent()
	local eventData: { [string]: any } = {
		inputType = inputType,
		action = action,
		timestamp = os.clock(),
	}

	if inputType == "keyboard" then
		-- Validate key
		if not VALID_KEYS[args.key] then
			error(`Invalid key '{args.key}'. Use KeyCode names like 'W', 'Space', 'LeftShift', etc.`)
		end
		eventData.key = args.key
		eventData.keyCode = Enum.KeyCode[args.key]

		if action == "tap" then
			-- Fire begin, wait briefly, then fire end
			eventData.action = "begin"
			inputEvent:Fire(eventData)
			task.wait(0.05)
			eventData.action = "end"
			inputEvent:Fire(eventData)
		else
			inputEvent:Fire(eventData)
		end

	elseif inputType == "mouse" then
		-- Validate mouse button
		local buttonEnum = VALID_MOUSE_BUTTONS[args.key]
		if not buttonEnum then
			error(`Invalid mouse button '{args.key}'. Must be 'Left', 'Right', or 'Middle'.`)
		end
		eventData.button = args.key
		eventData.buttonEnum = buttonEnum
		eventData.position = Vector2.new(args.mouse_x or 0, args.mouse_y or 0)

		if action == "tap" then
			-- Fire begin, wait briefly, then fire end (click)
			eventData.action = "begin"
			inputEvent:Fire(eventData)
			task.wait(0.05)
			eventData.action = "end"
			inputEvent:Fire(eventData)
		else
			inputEvent:Fire(eventData)
		end
	else
		error(`Invalid input_type '{args.input_type}'. Must be 'keyboard' or 'mouse'.`)
	end

	return HttpService:JSONEncode({
		success = true,
		inputType = inputType,
		key = args.key,
		action = args.action,
		eventName = "MCPInputEvent",
		eventLocation = "ReplicatedStorage.MCPInputEvent",
		note = "Event fired. Game scripts should connect to ReplicatedStorage.MCPInputEvent to handle simulated input.",
	})
end

local function handleSimulateInput(args: Types.ToolArgs): string?
	local inputArgs = args["SimulateInput"]
	if not inputArgs then
		return nil
	end

	return simulateInput(inputArgs)
end

return handleSimulateInput :: Types.ToolFunction

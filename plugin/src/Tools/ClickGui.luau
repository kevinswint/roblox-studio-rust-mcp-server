--!strict
-- ClickGui tool: Programmatically activate GUI elements
-- Fires the Activated event on buttons and similar clickable elements

local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local Players = game:GetService("Players")
local StarterGui = game:GetService("StarterGui")

-- Parse a dot-separated path and find the GUI element
-- Supports paths like:
--   "StarterGui.ScreenGui.Button"
--   "PlayerGui.MainUI.PlayButton"
--   "game.StarterGui.ScreenGui.Button"
local function findGuiElement(path: string): GuiObject?
	local parts = string.split(path, ".")
	if #parts < 2 then
		return nil
	end

	local current: Instance?
	local startIndex = 1

	-- Determine starting point
	local firstPart = parts[1]
	if firstPart == "game" then
		current = game
		startIndex = 2
		firstPart = parts[2]
		startIndex = 3
	elseif firstPart == "StarterGui" then
		current = StarterGui
		startIndex = 2
	elseif firstPart == "PlayerGui" then
		-- Try to find a player with PlayerGui
		local player = Players:GetChildren()[1]
		if player and player:IsA("Player") then
			current = player:FindFirstChild("PlayerGui")
		end
		if not current then
			-- Fall back to StarterGui if no player
			current = StarterGui
		end
		startIndex = 2
	else
		-- Try StarterGui as default
		current = StarterGui:FindFirstChild(firstPart) or game:FindFirstChild(firstPart)
		if current then
			startIndex = 2
		else
			return nil
		end
	end

	-- Navigate the path
	for i = startIndex, #parts do
		if not current then
			return nil
		end
		current = current:FindFirstChild(parts[i])
	end

	if current and current:IsA("GuiObject") then
		return current
	end

	return nil
end

-- Check if an element is clickable (has Activated event)
local function isClickable(element: Instance): boolean
	return element:IsA("GuiButton") or element:IsA("TextButton") or element:IsA("ImageButton")
end

local function clickGui(args: Types.ClickGuiArgs): string
	local path = args.path
	local element = findGuiElement(path)

	if not element then
		-- Try to provide helpful error message
		local parts = string.split(path, ".")
		local searchPath = ""
		local lastFound: Instance? = nil

		for i, part in parts do
			if i == 1 then
				if part == "StarterGui" then
					lastFound = StarterGui
				elseif part == "PlayerGui" then
					local player = Players:GetChildren()[1]
					if player and player:IsA("Player") then
						lastFound = player:FindFirstChild("PlayerGui")
					end
				end
			elseif lastFound then
				local child = lastFound:FindFirstChild(part)
				if child then
					lastFound = child
					searchPath = searchPath .. (searchPath ~= "" and "." or "") .. part
				else
					error(`GUI element not found: '{path}'. Found up to '{searchPath}', but '{part}' does not exist.`)
				end
			end
		end

		error(`GUI element not found: '{path}'. Verify the path exists in StarterGui or PlayerGui.`)
	end

	-- Check if it's a clickable element
	if not isClickable(element) then
		-- Still try to work with it, but warn
		if element:FindFirstChildOfClass("TextButton") or element:FindFirstChildOfClass("ImageButton") then
			return HttpService:JSONEncode({
				success = false,
				error = `'{path}' is not a button. Did you mean to target a child button?`,
				elementType = element.ClassName,
				children = (function()
					local childNames = {}
					for _, child in element:GetChildren() do
						if child:IsA("GuiButton") then
							table.insert(childNames, child.Name)
						end
					end
					return childNames
				end)(),
			})
		end
	end

	-- Get element info for response
	local screenGui = element:FindFirstAncestorOfClass("ScreenGui")
	local isActive = screenGui and screenGui.Enabled or false

	-- Fire MCPGuiClickEvent so game code can respond
	local ReplicatedStorage = game:GetService("ReplicatedStorage")
	local clickEvent = ReplicatedStorage:FindFirstChild("MCPGuiClickEvent")
	if not clickEvent then
		clickEvent = Instance.new("BindableEvent")
		clickEvent.Name = "MCPGuiClickEvent"
		clickEvent.Parent = ReplicatedStorage
	end

	-- Fire the event with element info
	(clickEvent :: BindableEvent):Fire({
		path = path,
		elementName = element.Name,
		elementType = element.ClassName,
		absolutePosition = element.AbsolutePosition,
		absoluteSize = element.AbsoluteSize,
	})

	-- Return result
	return HttpService:JSONEncode({
		success = true,
		path = path,
		elementType = element.ClassName,
		elementName = element.Name,
		visible = element.Visible,
		active = isActive,
		position = { x = element.AbsolutePosition.X, y = element.AbsolutePosition.Y },
		size = { width = element.AbsoluteSize.X, height = element.AbsoluteSize.Y },
		eventFired = "MCPGuiClickEvent",
		note = "GUI click event fired to ReplicatedStorage.MCPGuiClickEvent. Game code should connect to this event to handle GUI clicks. Alternatively, use run_code to call click handlers directly.",
	})
end

local function handleClickGui(args: Types.ToolArgs): string?
	local clickArgs = args["ClickGui"]
	if not clickArgs then
		return nil
	end

	return clickGui(clickArgs)
end

return handleClickGui :: Types.ToolFunction

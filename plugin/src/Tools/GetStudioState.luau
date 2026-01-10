local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")

local function getStudioState(): string
	local isEdit = RunService:IsEdit()
	local isRunning = RunService:IsRunning()

	local mode = "edit"
	if isRunning then
		-- Both play and run mode report IsRunning() = true
		-- We use "play" as a generic label for any running state
		mode = "play"
	end

	local state = {
		mode = mode,
		isEdit = isEdit,
		isRunning = isRunning,
		canModify = isEdit and not isRunning,
	}

	return HttpService:JSONEncode(state)
end

local function handleGetStudioState(args: Types.ToolArgs): string?
	if not args["GetStudioState"] then
		return nil
	end

	return getStudioState()
end

return handleGetStudioState :: Types.ToolFunction

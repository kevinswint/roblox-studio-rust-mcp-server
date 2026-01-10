local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local RunService = game:GetService("RunService")

local function startPlaytest(): string
	if RunService:IsRunning() then
		error("Studio is already in play/run mode. Stop simulation first.")
	end

	RunService:Run()
	return "Started playtest mode. Player character spawned."
end

local function handleStartPlaytest(args: Types.ToolArgs): string?
	if not args["StartPlaytest"] then
		return nil
	end

	return startPlaytest()
end

return handleStartPlaytest :: Types.ToolFunction

local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local RunService = game:GetService("RunService")

local function startSimulation(): string
	if RunService:IsRunning() then
		error("Studio is already in play/run mode. Stop simulation first.")
	end

	RunService:Run()
	return "Started simulation mode. Physics running without player."
end

local function stopSimulation(): string
	if not RunService:IsRunning() then
		error("Studio is already in edit mode.")
	end

	RunService:Stop()
	return "Stopped simulation. Returned to edit mode."
end

local function handleSimulation(args: Types.ToolArgs): string?
	if args["StartSimulation"] then
		return startSimulation()
	elseif args["StopSimulation"] or args["StopPlaytest"] then
		return stopSimulation()
	end
	return nil
end

return handleSimulation :: Types.ToolFunction

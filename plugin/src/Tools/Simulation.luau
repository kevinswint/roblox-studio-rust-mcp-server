local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local RunService = game:GetService("RunService")

local function startSimulation(): string
	-- Note: RunService:IsRunning() returns false from plugin context even during playtest
	-- so we can't reliably check current state. Just try to run and let it fail naturally.
	local success, err = pcall(function()
		RunService:Run()
	end)

	if success then
		return "Started simulation mode. Physics running without player."
	else
		error("Failed to start simulation: " .. tostring(err))
	end
end

local function stopSimulation(): string
	-- Note: RunService:IsRunning() returns false from plugin context even during playtest
	-- because plugin runs in separate DataModel. Just call Stop() and let it fail naturally.
	local success, err = pcall(function()
		RunService:Stop()
	end)

	if success then
		return "Stopped simulation/playtest. Returned to edit mode."
	else
		return "Stop requested. Note: " .. tostring(err)
	end
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

local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local RunService = game:GetService("RunService")
local StudioTestService = game:GetService("StudioTestService")

local function startPlaytest(): string
	if RunService:IsRunning() then
		error("Studio is already in play/run mode. Stop simulation first.")
	end

	-- Use task.spawn to avoid blocking since ExecutePlayModeAsync yields until test ends.
	-- This starts actual playtest mode (F5) with a player character, unlike RunService:Run()
	-- which only starts simulation mode (F8) without a player.
	task.spawn(function()
		pcall(function()
			StudioTestService:ExecutePlayModeAsync(nil)
		end)
	end)

	-- Give Studio a moment to transition to playtest mode
	task.wait(0.1)

	return "Started playtest mode with player character. Note: run_code commands will execute in edit-mode context during playtest."
end

local function handleStartPlaytest(args: Types.ToolArgs): string?
	if not args["StartPlaytest"] then
		return nil
	end

	return startPlaytest()
end

return handleStartPlaytest :: Types.ToolFunction

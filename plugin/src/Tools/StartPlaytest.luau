local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")

-- Try to get StudioTestService (may not exist in older Studio versions)
local StudioTestService = nil
pcall(function()
	StudioTestService = game:GetService("StudioTestService")
end)

local function startPlaytest(): string
	-- Note: RunService:IsRunning() returns false from plugin context even during playtest
	-- because plugin runs in separate DataModel. We can't reliably check current state.

	-- Try StudioTestService first (starts with player character)
	if StudioTestService then
		-- Use task.defer to schedule after current execution cycle
		-- This allows the HTTP response to complete before the yielding call
		task.defer(function()
			local success, err = pcall(function()
				-- Pass empty table instead of nil to avoid potential argument issues
				StudioTestService:ExecutePlayModeAsync({})
			end)
			if not success then
				warn("[MCP] ExecutePlayModeAsync failed:", err)
			end
		end)

		-- Give a moment for the deferred call to be scheduled
		task.wait(0.05)

		-- We can't verify if playtest started because IsRunning() returns false
		-- from plugin context. Trust that the call was made successfully.
		return HttpService:JSONEncode({
			success = true,
			mode = "playtest",
			note = "Started playtest mode (F5) via StudioTestService. Note: State cannot be verified from plugin context.",
			hasPlayer = true,
		})
	end

	-- Fallback: StudioTestService not available, use RunService:Run() (simulation mode)
	local success, err = pcall(function()
		RunService:Run()
	end)

	if success then
		task.wait(0.1)
		return HttpService:JSONEncode({
			success = true,
			mode = "simulation",
			note = "Started simulation mode (F8) - StudioTestService not available.",
			hasPlayer = false,
		})
	end

	error("Failed to start playtest. StudioTestService not available. RunService:Run() failed: " .. tostring(err))
end

local function handleStartPlaytest(args: Types.ToolArgs): string?
	if not args["StartPlaytest"] then
		return nil
	end

	return startPlaytest()
end

return handleStartPlaytest :: Types.ToolFunction

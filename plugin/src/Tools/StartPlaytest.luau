local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local HttpService = game:GetService("HttpService")
local RunService = game:GetService("RunService")

-- Try to get StudioTestService (may not exist in older Studio versions)
local StudioTestService = nil
pcall(function()
	StudioTestService = game:GetService("StudioTestService")
end)

-- Configuration
local VERIFICATION_TIMEOUT = 10 -- seconds to wait for state change
local POLL_INTERVAL = 0.1 -- seconds between state checks

-- Wait for RunService state to change with timeout
local function waitForState(targetRunning: boolean, timeout: number): (boolean, string?)
	local startTime = os.clock()

	while os.clock() - startTime < timeout do
		local isRunning = RunService:IsRunning()
		if isRunning == targetRunning then
			return true, nil
		end
		task.wait(POLL_INTERVAL)
	end

	return false, string.format(
		"Timeout after %.1fs waiting for state change (expected IsRunning=%s, got %s)",
		timeout, tostring(targetRunning), tostring(RunService:IsRunning())
	)
end

local function startPlaytest(): string
	-- Note: We cannot reliably check if already running because RunService:IsRunning()
	-- returns false from plugin context during playtest due to DataModel isolation.
	-- StudioTestService will return an error if playtest is already running.

	-- Try StudioTestService first (starts with player character)
	if StudioTestService then
		-- Use task.spawn because ExecutePlayModeAsync yields/blocks until playtest starts.
		-- We need to return the HTTP response immediately, not wait for playtest to fully load.
		local startError = nil
		task.spawn(function()
			local success, err = pcall(function()
				StudioTestService:ExecutePlayModeAsync({})
			end)
			if not success then
				startError = tostring(err)
				warn("[MCP] ExecutePlayModeAsync failed: " .. startError)
			end
		end)

		-- Give it a moment to check for immediate errors (like "already running")
		task.wait(0.5)

		if startError then
			return HttpService:JSONEncode({
				success = false,
				error = "ExecutePlayModeAsync failed: " .. startError,
				suggestion = "Try stopping any pending playtest operations first.",
			})
		end

		-- Command was sent - playtest should be starting
		return HttpService:JSONEncode({
			success = true,
			mode = "playtest",
			hasPlayer = true,
			note = "Started playtest mode (F5) via StudioTestService. State cannot be verified from plugin context.",
		})
	end

	-- Fallback: StudioTestService not available, use RunService:Run() (simulation mode)
	local success, err = pcall(function()
		RunService:Run()
	end)

	if not success then
		return HttpService:JSONEncode({
			success = false,
			error = "RunService:Run() failed: " .. tostring(err),
		})
	end

	-- For simulation mode, RunService:IsRunning() DOES work from plugin context
	-- because simulation runs in the same DataModel
	local verified, verifyErr = waitForState(true, VERIFICATION_TIMEOUT)

	if verified then
		return HttpService:JSONEncode({
			success = true,
			verified = true,
			mode = "simulation",
			note = "Started simulation mode (F8) - StudioTestService not available.",
			hasPlayer = false,
		})
	else
		return HttpService:JSONEncode({
			success = false,
			error = "Simulation started but verification failed: " .. tostring(verifyErr),
		})
	end
end

local function handleStartPlaytest(args: Types.ToolArgs): string?
	if not args["StartPlaytest"] then
		return nil
	end

	return startPlaytest()
end

return handleStartPlaytest :: Types.ToolFunction

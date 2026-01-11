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
	if RunService:IsRunning() then
		error("Studio is already in play/run mode. Stop simulation first.")
	end

	-- Track if playtest started successfully
	local startError: string? = nil
	local playtestStarted = false

	-- Try StudioTestService first (starts with player character)
	if StudioTestService then
		-- Use task.defer to schedule after current execution cycle
		-- This should allow the HTTP response to complete before yielding
		task.defer(function()
			local success, err = pcall(function()
				-- Pass empty table instead of nil to avoid potential argument issues
				StudioTestService:ExecutePlayModeAsync({})
			end)
			if not success then
				warn("[MCP] ExecutePlayModeAsync failed:", err)
			end
		end)

		-- Wait briefly to see if playtest starts
		task.wait(0.3)

		-- Check if we're now running
		if RunService:IsRunning() then
			playtestStarted = true
		else
			startError = "ExecutePlayModeAsync did not start playtest (may yield in deferred context)"
		end
	else
		startError = "StudioTestService not available"
	end

	-- If StudioTestService failed, fall back to RunService:Run() (simulation mode)
	if not playtestStarted then
		local success, err = pcall(function()
			RunService:Run()
		end)

		if success then
			-- Note: RunService:IsRunning() may return false from plugin context
			-- even when simulation is actually running. Trust the pcall success.
			task.wait(0.1)
			return HttpService:JSONEncode({
				success = true,
				mode = "simulation",
				note = "Started simulation mode (F8) instead of playtest. StudioTestService issue: " .. (startError or "unknown"),
				hasPlayer = false,
				warning = "RunService:Run() called successfully. If simulation didn't start, try pressing F8 manually.",
			})
		end

		error("Failed to start any mode. StudioTestService: " .. (startError or "N/A") .. ". RunService: " .. tostring(err))
	end

	return HttpService:JSONEncode({
		success = true,
		mode = "playtest",
		note = "Started playtest mode (F5) with player character via StudioTestService.",
		hasPlayer = true,
	})
end

local function handleStartPlaytest(args: Types.ToolArgs): string?
	if not args["StartPlaytest"] then
		return nil
	end

	return startPlaytest()
end

return handleStartPlaytest :: Types.ToolFunction

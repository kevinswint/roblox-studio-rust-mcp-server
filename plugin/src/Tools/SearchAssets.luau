local Main = script:FindFirstAncestor("MCPStudioPlugin")
local Types = require(Main.Types)

local InsertService = game:GetService("InsertService")
local HttpService = game:GetService("HttpService")

type SearchAssetsArgs = {
	query: string,
	max_results: number?,
}

type AssetResult = {
	assetId: number,
	name: string,
	creator: string,
}

local function handleSearchAssets(args: Types.ToolArgs): string?
	if not args["SearchAssets"] then
		return nil
	end

	local searchArgs: SearchAssetsArgs = args["SearchAssets"]

	if type(searchArgs.query) ~= "string" or searchArgs.query == "" then
		return "[ERROR] Missing or empty query parameter"
	end

	local maxResults = math.min(searchArgs.max_results or 10, 20)

	-- Search for assets using InsertService
	local success, searchResults = pcall(function()
		return InsertService:GetFreeModels(searchArgs.query, 0)
	end)

	if not success then
		return "[ERROR] Failed to search assets: " .. tostring(searchResults)
	end

	-- Parse results
	local assets: {AssetResult} = {}

	if searchResults and searchResults[1] and searchResults[1].Results then
		for i, result in searchResults[1].Results do
			if i > maxResults then
				break
			end
			table.insert(assets, {
				assetId = result.AssetId,
				name = result.Name,
				creator = result.CreatorName or "Unknown",
			})
		end
	end

	if #assets == 0 then
		return "[WARNING] No assets found for query: " .. searchArgs.query
	end

	-- Format results
	local lines = {
		string.format("[SUCCESS] Found %d assets for '%s':", #assets, searchArgs.query),
		"",
	}

	for i, asset in assets do
		table.insert(lines, string.format(
			"%d. **%s** (ID: %d) by %s",
			i, asset.name, asset.assetId, asset.creator
		))
	end

	table.insert(lines, "")
	table.insert(lines, "Use preview_asset with an asset_id to see what it looks like before inserting.")
	table.insert(lines, "Use insert_model with the name to insert directly (uses first search result).")

	-- Also return JSON for programmatic use
	table.insert(lines, "")
	table.insert(lines, "```json")
	table.insert(lines, HttpService:JSONEncode({ assets = assets, query = searchArgs.query }))
	table.insert(lines, "```")

	return table.concat(lines, "\n")
end

return handleSearchAssets :: Types.ToolFunction

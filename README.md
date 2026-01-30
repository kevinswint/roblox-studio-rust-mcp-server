# Roblox Studio MCP Server (Enhanced Fork)

> **Fork of [Roblox/studio-rust-mcp-server](https://github.com/Roblox/studio-rust-mcp-server) with additional tools for AI-assisted game development.**

This fork adds new MCP tools that enable AI assistants to create complete, functional Roblox games. The upstream repository has not merged community contributions since July 2025.

---

## ⚠️ IMPORTANT: Playtest Control Limitations

**The MCP plugin runs in a separate DataModel from playtest mode.** This causes critical limitations:

| Feature | Simulation (F8) | Playtest (F5) |
|---------|-----------------|---------------|
| `start_simulation` | ✅ Works | N/A |
| `start_playtest` | N/A | ✅ Works |
| `stop_simulation` | ✅ Works | ❌ No effect |
| `stop_playtest` | ✅ Works | ❌ No effect |
| `run_code` | ✅ Full access | ⚠️ Plugin context only |
| `run_server_code` | N/A | ✅ Requires MCPServerCodeRunner |

### To Enable Full Playtest Control

Add **MCPServerCodeRunner** to your game's `ServerScriptService`:

1. Copy `MCPServerCodeRunner.lua` from this repository to `ServerScriptService`
2. Enable HttpService: Game Settings → Security → Allow HTTP Requests
3. Now you can:
   - Stop playtest: `run_server_code({ code = "game:GetService('StudioTestService'):EndTest('done')" })`
   - Execute server-side code during playtest
   - Access server-side `_G` values and game state

**Without MCPServerCodeRunner**, you must manually press F6 or the Stop button to end playtest.

---

## Additional Tools in This Fork

### `write_script`

Creates or updates Script, LocalScript, or ModuleScript instances with provided Luau source code.

**Why this matters:** The official MCP server can only execute code via `run_code`, but cannot create or modify script source directly. This is because Roblox blocks direct `Script.Source` access. This tool uses `ScriptEditorService:UpdateSourceAsync()` - the officially supported method for writing script source in Studio plugins.

**Parameters:**
- `path` - Path to script in game hierarchy (e.g., `ServerScriptService.Managers.GameManager`)
- `source` - The Luau source code to write
- `script_type` (optional) - Type of script to create: `"Script"`, `"LocalScript"`, or `"ModuleScript"`. Defaults to `"Script"`. Only used when creating new scripts.

**Features:**
- Creates scripts in any service (ServerScriptService, ReplicatedStorage, StarterPlayerScripts, etc.)
- Automatically creates intermediate folders for nested paths
- Updates existing scripts with new source code

**Example:**
```
write_script({
  path: "ServerScriptService.GameManager",
  source: "print('Hello from AI!')"
})

write_script({
  path: "ReplicatedStorage.Utils.MathHelpers",
  source: "local M = {} ... return M",
  script_type: "ModuleScript"
})
```

**Status:** [PR #52](https://github.com/Roblox/studio-rust-mcp-server/pull/52) submitted to upstream

---

### `capture_screenshot`

Captures a screenshot of the Roblox Studio window and returns it as a JPEG image.

**Why this matters:** Enables AI assistants to visually inspect the game state, verify UI changes, debug visual issues, and analyze workspace layouts without manual screenshots.

**Parameters:** None

**Features:**
- Captures the Studio window directly (no plugin communication needed)
- Supports macOS and Windows
- Returns high-quality JPEG (up to 1920px, quality 85)
- Requires Screen Recording permission on macOS

**Example:**
```
capture_screenshot({})
```

---

### `read_output`

Reads captured output from Roblox Studio's Output window.

**Why this matters:** Allows AI assistants to check for script errors, review print statements, and debug issues without manual intervention.

**Parameters:**
- `filter` (optional) - Filter by level: `"all"` (default), `"print"`, `"warn"`, or `"error"`
- `max_lines` (optional) - Maximum lines to return (default: 1000, max: 10000)
- `clear_after_read` (optional) - Clear buffer after reading (default: true)

**Features:**
- Captures print(), warn(), and error() messages during Edit and Play modes
- Persistent buffer survives mode transitions
- FIFO eviction with overflow warnings
- Holds up to 10,000 messages

**Example:**
```
read_output({ filter: "error", max_lines: 100 })
```

---

### `get_studio_state`

Gets the current Studio mode (edit/play/run) to determine if workspace modifications are safe.

**Parameters:** None

**Returns:** JSON with mode, isEdit, isRunning, and canModify flags

**Example:**
```
get_studio_state({})
// Returns: {"mode":"edit","isEdit":true,"isRunning":false,"canModify":true}
```

---

### `start_playtest` / `start_simulation`

Starts playtest or simulation mode.

- `start_playtest` - Starts play mode (F5) with a player character
- `start_simulation` - Starts run mode (F8) without a player (physics only)

**Parameters:** None

**Example:**
```
start_playtest({})
```

---

### `stop_simulation` / `stop_playtest`

Stops playtest/simulation and returns to edit mode.

**Parameters:** None

**⚠️ Limitation:** `stop_playtest` only works for **simulation mode (F8)**, not playtest mode (F5). See [Playtest Control Limitations](#️-important-playtest-control-limitations) above.

**To stop playtest programmatically:**
```
run_server_code({ code = "game:GetService('StudioTestService'):EndTest('done')" })
```
Requires MCPServerCodeRunner in your game.

---

### `simulate_input`

Simulates keyboard or mouse input during playtest via HTTP polling.

**Why this matters:** Enables automated testing of gameplay that requires player input. Commands are queued and polled by the game.

**Parameters:**
- `input_type` - `"keyboard"` or `"mouse"`
- `key` - Key name (`"W"`, `"Space"`, `"E"`) or mouse button (`"Left"`, `"Right"`)
- `action` - `"begin"`, `"end"`, or `"tap"`
- `mouse_x`, `mouse_y` (optional) - Mouse position for mouse input

**Supported Keys:** A-Z, Space, Return, Tab, Escape, LeftShift, LeftControl, Arrow keys, F1-F12

**Example:**
```
simulate_input({ input_type: "keyboard", key: "E", action: "tap" })
```

**Requires:** Game must include MCPInputPoller scripts (see below)

---

### `click_gui`

Simulates clicking a GUI element during playtest via HTTP polling.

**Parameters:**
- `path` - Path to GUI element (e.g., `"FluxUI.WelcomeMessage.PlayButton"`)

**Example:**
```
click_gui({ path: "ScreenGui.PlayButton" })
```

**Requires:** Game must include MCPInputPoller scripts (see below)

---

### `move_character`

Moves or teleports a character in the workspace.

**Parameters:**
- `x`, `y`, `z` - Target world coordinates
- `instant` (optional) - `true` for teleport, `false` for walk
- `character_name` (optional) - Specific character to move

**Example:**
```
move_character({ x: 0, y: 5, z: 10, instant: true, character_name: "Aria" })
```

---

### `run_server_code`

Executes Luau code in the **server context** during playtest (not the plugin context).

**Why this matters:** `run_code` executes in the plugin's DataModel, which is isolated from the running game during playtest. `run_server_code` executes in the actual game server, giving access to:
- Server-side `_G` values set by your scripts
- DataStores and other server-only services
- The ability to call `StudioTestService:EndTest()` to stop playtest
- Full game state from the server perspective

**Parameters:**
- `code` - Luau code to execute

**Examples:**
```lua
-- Check a server-side global value
run_server_code({ code = "return _G.GameState" })

-- Get all players
run_server_code({ code = "return game.Players:GetPlayers()" })

-- Stop playtest programmatically
run_server_code({ code = "game:GetService('StudioTestService'):EndTest('done')" })
```

**Requires:** MCPServerCodeRunner script in ServerScriptService. See [Server Code Execution Setup](#server-code-execution-setup) below.

### `validate_ui`

Scans UI for common responsive layout issues. Returns a JSON report of problems found.

**Parameters:**
- `path` - Optional path to ScreenGui (e.g., `"StarterGui.MainUI"`). If not specified, validates all ScreenGuis.

**Checks for:**
- **Overlapping elements** - GUI elements that visually overlap
- **Offscreen elements** - Elements extending beyond viewport boundaries
- **Pixel positioning** - Using Offset without Scale (not responsive)
- **Missing constraints** - Containers without UISizeConstraint
- **Anchor mismatches** - AnchorPoint doesn't match Position alignment

**Example prompt:** "Validate my UI for layout issues"

### `create_responsive_layout`

Creates a ScreenGui with best-practice responsive container structure.

**Parameters:**
- `name` - Name for the ScreenGui (e.g., `"MainUI"`)
- `containers` - Array of positions: `"TopLeft"`, `"TopRight"`, `"TopCenter"`, `"BottomLeft"`, `"BottomRight"`, `"BottomCenter"`, `"CenterLeft"`, `"CenterRight"`, `"Center"`

**Each container includes:**
- Correct `AnchorPoint` and `Position` for its location
- `UISizeConstraint` (Min 100x50, Max 400x600)
- `UIListLayout` for automatic child arrangement
- `UIPadding` for internal spacing

**Example prompt:** "Create a responsive UI with containers in the top-left and bottom-center"

### `preview_layout`

Calculates what UI would look like at a specific viewport size without using Device Emulator.

**Parameters:**
- `width` - Target viewport width in pixels (e.g., `390` for iPhone 14)
- `height` - Target viewport height in pixels (e.g., `844` for iPhone 14)
- `path` - Optional path to ScreenGui. If not specified, previews all ScreenGuis.

**Returns JSON with:**
- Element positions and sizes at target viewport
- `offscreen` flag for elements extending beyond viewport
- `clipped` flag for partially visible elements
- Summary of total elements and issues found

**Example prompt:** "Preview my UI at iPhone 14 dimensions (390x844)"

---

## Server Code Execution Setup

To enable `run_server_code` and programmatic playtest stopping, add **MCPServerCodeRunner** to your game:

1. **Enable HttpService:** Game Settings → Security → Allow HTTP Requests

2. **Add MCPServerCodeRunner** (Script) to `ServerScriptService`:
   - Copy `MCPServerCodeRunner.lua` from this repository
   - The script polls `localhost:44755/mcp/server_code` for commands
   - Executes code using `loadstring()` and returns results

**Verification:**
- Start playtest (F5)
- Check output for `[MCPServerCodeRunner] Starting server code poll loop`
- Test with: `run_server_code({ code = "return 'Hello from server!'" })`

**Security Note:** MCPServerCodeRunner executes arbitrary code. Only use during local development. Do NOT include in published games.

---

## Input Simulation Setup

Input simulation (`simulate_input`, `click_gui`) uses HTTP polling because:
- HTTP requests can only be made from ServerScripts
- Input must be executed on the client
- Roblox's DataModel isolation prevents direct communication during playtest

**Setup:**

1. Enable HttpService: Game Settings → Security → Allow HTTP Requests
2. Use `simulate_input` or `click_gui` - **scripts are auto-installed** on first use
3. Restart playtest (F5) to load the newly installed scripts

**Auto-installed Scripts:**

| Script | Location | Purpose |
|--------|----------|---------|
| MCPInputPoller | ServerScriptService | Polls `localhost:44755/mcp/input` for commands, relays to clients |
| MCPInputHandler | StarterPlayerScripts | Receives commands, fires `MCPInputReceived` BindableEvent |
| MCPMovementHandler | StarterPlayerScripts | Translates WASD/Space input into character movement |
| MCPClickSupport | ReplicatedStorage | ModuleScript for handling both real and MCP GUI clicks |

**MCPClickSupport Usage:**

For buttons that need to respond to both real clicks and `click_gui`:

```lua
local MCP = require(game.ReplicatedStorage.MCPClickSupport)

MCP.onClick(button, function()
    print("Button clicked!")
end)
```

This replaces `button.MouseButton1Click:Connect()` and handles both input sources.

**Custom Ability Integration:**

To trigger abilities via MCP input, listen to the `MCPInputReceived` BindableEvent:

```lua
local mcpEvent = game.ReplicatedStorage:WaitForChild("MCPInputReceived")
mcpEvent.Event:Connect(function(inputInfo)
    if inputInfo.KeyCode == Enum.KeyCode.E and inputInfo.UserInputState == Enum.UserInputState.Begin then
        -- Trigger ability
    end
end)
```

**Verification:**
- Check output for `[MCPPoller] Started - polling`
- Check output for `[MCPInput] Client handler ready!`
- Check output for `[MCPMovement] Handler ready - WASD and Space supported`
- Send `simulate_input` and verify `[MCPPoller] Got 1 commands!`

---

## Upstream Tools

This fork includes all tools from the official repository:

- **`run_code`** - Execute Luau code in Studio and capture output
- **`insert_model`** - Insert models from the Roblox marketplace

---

## Installation

### Build from source (recommended for this fork)

1. Ensure you have [Roblox Studio](https://create.roblox.com/docs/en-us/studio/setup) and [Claude Desktop](https://claude.ai/download) or [Claude Code](https://claude.ai/code) installed.
2. Exit Claude and Roblox Studio if running.
3. [Install Rust](https://www.rust-lang.org/tools/install).
4. Clone this repository:
   ```sh
   git clone https://github.com/kevinswint/roblox-studio-rust-mcp-server.git
   cd roblox-studio-rust-mcp-server
   ```
5. Build and install:
   ```sh
   cargo run
   ```

This builds the MCP server, installs the Studio plugin, and configures Claude.

### Verify setup

1. Open Roblox Studio and check the **Plugins** tab for the MCP plugin
2. In Claude, verify tools are available: `run_code`, `insert_model`, `write_script`, `capture_screenshot`, `read_output`, `get_studio_state`, `start_playtest`, `start_simulation`, `stop_simulation`, `stop_playtest`

---

## Keeping in Sync with Upstream

This fork stays up-to-date with the official repository:

```sh
git fetch upstream
git merge upstream/main
```

---

## Contributing

Contributions welcome! If you have improvements:

1. Consider submitting PRs to [upstream](https://github.com/Roblox/studio-rust-mcp-server) first
2. If upstream is unresponsive, PRs to this fork are welcome

---

## Future Enhancement Ideas

The following features would improve the MCP workflow but are not yet implemented:

### 1. Reliable Playtest Stop (F6 equivalent)

**Problem:** `stop_playtest` only works for simulation mode (F8), not playtest mode (F5). Users must manually press F6 to stop playtest.

**Potential Solution:**
- Expose `StudioTestService:EndTest()` more reliably from the plugin context
- Or create a native Rust command that sends keystrokes (F6) to the Studio window
- The `run_server_code` workaround exists but requires MCPServerCodeRunner setup

### 2. Restart Studio Programmatically

**Problem:** Sometimes Studio needs a full restart (stale state, plugin issues, etc.). Currently requires manual intervention.

**Potential Solution:**
- Create an external helper that can:
  - Close Roblox Studio gracefully
  - Relaunch Studio with the same place file
  - Wait for MCP plugin to reconnect
- On macOS: Use AppleScript or `osascript` commands
- On Windows: Use PowerShell or native Windows APIs
- Would need to persist the current place file path for reopening

---

## Original README

*The following is from the original Roblox repository:*

---

# Roblox Studio MCP Server

This repository contains a reference implementation of the Model Context Protocol (MCP) that enables
communication between Roblox Studio via a plugin and [Claude Desktop](https://claude.ai/download) or [Cursor](https://www.cursor.com/).
It consists of the following Rust-based components, which communicate through internal shared
objects.

- A web server built on `axum` that a Studio plugin long polls.
- A `rmcp` server that talks to Claude via `stdio` transport.

When LLM requests to run a tool, the plugin will get a request through the long polling and post a
response. It will cause responses to be sent to the Claude app.

**Please note** that this MCP server will be accessed by third-party tools, allowing them to modify
and read the contents of your opened place. Third-party data handling and privacy practices are
subject to their respective terms and conditions.

![Scheme](MCP-Server.png)

The setup process also contains a short plugin installation and Claude Desktop configuration script.

## Setup

### Install with release binaries

This MCP Server supports pretty much any MCP Client but will automatically set up only [Claude Desktop](https://claude.ai/download) and [Cursor](https://www.cursor.com/) if found.

To set up automatically:

1. Ensure you have [Roblox Studio](https://create.roblox.com/docs/en-us/studio/setup),
   and [Claude Desktop](https://claude.ai/download)/[Cursor](https://www.cursor.com/) installed and started at least once.
1. Exit MCP Clients and Roblox Studio if they are running.
1. Download and run the installer:
   1. Go to the [releases](https://github.com/Roblox/studio-rust-mcp-server/releases) page and
      download the latest release for your platform.
   1. Unzip the downloaded file if necessary and run the installer.
   1. Restart Claude/Cursor and Roblox Studio if they are running.

### Setting up manually

To set up manually add following to your MCP Client config:

```json
{
  "mcpServers": {
    "Roblox Studio": {
      "args": [
        "--stdio"
      ],
      "command": "Path-to-downloaded\\rbx-studio-mcp.exe"
    }
  }
}
```

On macOS the path would be something like `"/Applications/RobloxStudioMCP.app/Contents/MacOS/rbx-studio-mcp"` if you move the app to the Applications directory.

### Build from source

To build and install the MCP reference implementation from this repository's source code:

1. Ensure you have [Roblox Studio](https://create.roblox.com/docs/en-us/studio/setup) and
   [Claude Desktop](https://claude.ai/download) installed and started at least once.
1. Exit Claude and Roblox Studio if they are running.
1. [Install](https://www.rust-lang.org/tools/install) Rust.
1. Download or clone this repository.
1. Run the following command from the root of this repository.
   ```sh
   cargo run
   ```
   This command carries out the following actions:
      - Builds the Rust MCP server app.
      - Sets up Claude to communicate with the MCP server.
      - Builds and installs the Studio plugin to communicate with the MCP server.

After the command completes, the Studio MCP Server is installed and ready for your prompts from
Claude Desktop.

## Verify setup

To make sure everything is set up correctly, follow these steps:

1. In Roblox Studio, click on the **Plugins** tab and verify that the MCP plugin appears. Clicking on
   the icon toggles the MCP communication with Claude Desktop on and off, which you can verify in
   the Roblox Studio console output.
1. In the console, verify that `The MCP Studio plugin is ready for prompts.` appears in the output.
   Clicking on the plugin's icon toggles MCP communication with Claude Desktop on and off,
   which you can also verify in the console output.
1. Verify that Claude Desktop is correctly configured by clicking on the hammer icon for MCP tools
   beneath the text field where you enter prompts. This should open a window with the list of
   available Roblox Studio tools (`insert_model` and `run_code`).

**Note**: You can fix common issues with setup by restarting Studio and Claude Desktop. Claude
sometimes is hidden in the system tray, so ensure you've exited it completely.

## Send requests

1. Open a place in Studio.
1. Type a prompt in Claude Desktop and accept any permissions to communicate with Studio.
1. Verify that the intended action is performed in Studio by checking the console, inspecting the
   data model in Explorer, or visually confirming the desired changes occurred in your place.

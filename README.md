# Roblox Studio MCP Server (Enhanced Fork)

> **Fork of [Roblox/studio-rust-mcp-server](https://github.com/Roblox/studio-rust-mcp-server) with additional tools for AI-assisted game development.**

This fork adds new MCP tools that enable AI assistants to create complete, functional Roblox games. The upstream repository has not merged community contributions since July 2025.

## Additional Tools in This Fork

### `write_script`

Creates or updates Script, LocalScript, or ModuleScript instances with provided Luau source code.

**Why this matters:** The official MCP server can only execute code via `run_code`, but cannot create or modify script source directly. This is because Roblox blocks direct `Script.Source` access. This tool uses `ScriptEditorService:UpdateSourceAsync()` - the officially supported method for writing script source in Studio plugins.

**Parameters:**
- `path` - Path to script in game hierarchy (e.g., `ServerScriptService.Managers.GameManager`)
- `source` - The Luau source code to write

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
- Returns high-quality JPEG (up to 4096px, quality 85)
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

- `start_playtest` - Starts play mode with a player character
- `start_simulation` - Starts run mode without a player (physics only)

**Parameters:** None

---

### `stop_simulation` / `stop_playtest`

Stops playtest/simulation and returns to edit mode.

**Parameters:** None

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

## Input Simulation Setup

Input simulation (`simulate_input`, `click_gui`) uses HTTP polling because:
- HTTP requests can only be made from ServerScripts
- Input must be executed on the client
- Roblox's DataModel isolation prevents direct communication during playtest

**Installation:**

1. Enable HttpService: Game Settings > Security > Allow HTTP Requests

2. Add **MCPInputPoller** (Script) to `ServerScriptService`:
   ```lua
   -- Polls localhost:44755/mcp/input and relays commands to clients
   ```

3. Add **MCPInputHandler** (LocalScript) to `StarterPlayerScripts`:
   ```lua
   -- Receives commands and executes input/GUI clicks
   ```

See `MCPInputPoller.lua` in this repository for complete script code.

**Verification:**
- Check output for `[MCPPoller] Server started`
- Check output for `[MCPInput] Client handler ready!`
- Send `simulate_input` and verify `[MCPPoller] VERIFIED RECEIVED`

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

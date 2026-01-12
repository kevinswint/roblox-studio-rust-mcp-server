# Roblox Studio MCP Server

## Project Overview
MCP (Model Context Protocol) server that bridges Claude Code with Roblox Studio. Enables AI-assisted game development by providing tools to read/modify the game hierarchy, execute code, capture screenshots, and control playtest.

## Architecture
- **Rust server** (`src/`) - MCP protocol handler, HTTP server on port 44755
- **Luau plugin** (`plugin/`) - Runs inside Roblox Studio, communicates with Rust server
- **Helper scripts** (`MCPServerCodeRunner.lua`, `MCPInputPoller.lua`) - Optional scripts for games

## Key Limitation: DataModel Isolation
The plugin runs in a separate DataModel from playtest mode. This means:
- `run_code` executes in plugin context (can't see game state during playtest)
- `stop_playtest` only works for simulation mode, not playtest mode
- Use `run_server_code` with MCPServerCodeRunner for server-side access during playtest

## Build Commands
```bash
cargo build              # Debug build
cargo build --release    # Release build (for testing with Claude Code)
cargo test              # Run tests
```

## Testing Workflow
1. Build release: `cargo build --release`
2. Restart Claude Code (picks up new binary)
3. Open Roblox Studio with a place that has MCPServerCodeRunner in ServerScriptService
4. Enable LoadStringEnabled in ServerScriptService Properties panel
5. Start playtest and test tools

## MCP Tools
| Tool | Context | Notes |
|------|---------|-------|
| `run_code` | Plugin | Works in edit mode, limited during playtest |
| `run_server_code` | Server | Requires MCPServerCodeRunner, works during playtest |
| `start_playtest` | Plugin | Starts F5 mode |
| `stop_playtest` | Plugin | Only stops simulation (F8), not playtest (F5) |
| `fire_remote` | Server | ToClient/ToAllClients only, ToServer not supported |
| `simulate_input` | Client | Requires MCPInputPoller LocalScript |

## MCPServerCodeRunner Built-in Commands
Work without LoadStringEnabled:
- `STOP` - Stops playtest via StudioTestService:EndTest()
- `PING` - Returns "pong"
- `PLAYERS` - Lists connected players
- `STATE` - Returns server state JSON

## Git Workflow
- Fork: kevinswint/roblox-studio-rust-mcp-server
- Upstream: Roblox/studio-rust-mcp-server
- PRs go from fork branches to upstream main

## Common Issues
- **Stale MCP server**: Server now auto-kills old processes on startup
- **Rate limiting**: MCPServerCodeRunner polls at 500ms to avoid Roblox HTTP limits
- **loadstring disabled**: Use built-in commands or enable LoadStringEnabled in Properties

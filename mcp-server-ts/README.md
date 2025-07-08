# Para MCP Server (TypeScript)

A Model Context Protocol (MCP) server for Para, implemented in TypeScript using the official MCP SDK. This server provides para functionality to Claude Desktop, Claude Code, and other MCP clients.

## Features

- **Tools**: Start, finish, dispatch, list, recover para sessions, and show configuration
- **Resources**: Access current session info and para configuration
- **Reliable**: Uses official TypeScript MCP SDK for robust protocol handling
- **Hybrid**: Calls the Rust para binary for actual functionality

## Installation

```bash
cd mcp-server-ts
npm install
npm run build
```

## Usage

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "para": {
      "command": "node",
      "args": ["/path/to/para/mcp-server-ts/build/para-mcp-server.js"]
    }
  }
}
```

### Claude Code

```bash
# Add via CLI
claude mcp add para node /path/to/para/mcp-server-ts/build/para-mcp-server.js

# Or create .mcp.json in project root
{
  "mcpServers": {
    "para": {
      "type": "stdio",
      "command": "node",
      "args": ["/path/to/para/mcp-server-ts/build/para-mcp-server.js"]
    }
  }
}
```

## Available Tools

- `para_start` - Start new isolated session
- `para_finish` - Complete current session with commit message
- `para_list` - List all active sessions
- `para_recover` - Recover previous session
- `para_config_show` - Display current configuration

## Available Resources

- `para://current-session` - Current session information
- `para://config` - Para configuration

## Development

```bash
# Watch mode
npm run dev

# Clean build
npm run clean && npm install && npm run build

# Test
npm test
```

## Requirements

- Node.js 18+
- Para binary installed at `/Users/$(whoami)/.local/bin/para`
- TypeScript (dev dependency)
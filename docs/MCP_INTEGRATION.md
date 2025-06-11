# Para MCP Integration Guide

Para now supports Model Context Protocol (MCP) integration, allowing AI agents to use para commands directly through supported editors and AI tools.

## Quick Setup

### Automatic Setup (Recommended)
```bash
brew install para  # Installs both CLI and MCP server
just mcp-setup     # Auto-configures MCP integration
```

### Manual Installation
```bash
# Build from source
just install        # Installs para + para-mcp-server to ~/.local/bin/
just mcp-setup      # Configure MCP integration
```

## Editor Integration

### Claude Code
```bash
# Automatic setup (if Claude Code is installed)
claude mcp add para-server para-mcp-server

# Manual verification
claude mcp list
```

### VSCode/Cursor
Create `.cursor/mcp.json` in your project or workspace:
```json
{
  "mcpServers": {
    "para": {
      "command": "para-mcp-server"
    }
  }
}
```

### Roo Code (VSCode Extension)
1. Open Roo Code settings
2. Navigate to MCP settings panel
3. Para server should be auto-detected
4. Enable if not already active

## Available MCP Tools

The para MCP server exposes these tools to AI agents:

### Session Management
- **`para_start`** - Start new session
  - `name` (optional): Session name
  - `prompt` (optional): Initial prompt
  
- **`para_finish`** - Complete current session
  - `message` (required): Commit message

- **`para_dispatch`** - Create AI agent session
  - `name` (required): Session name
  - `prompt` (required): Task prompt
  - `file` (optional): Path to prompt file

### Session Operations
- **`para_list`** - List all sessions
- **`para_recover`** - Recover previous session
  - `name` (required): Session name to recover
  
- **`para_config_show`** - Show current configuration

## Available MCP Resources

AI agents can read these resources for context:

- **`para://current-session`** - Current session state
- **`para://available-sessions`** - List of all sessions
- **`para://config`** - Para configuration

## Parallel AI Development Orchestration

Para's MCP integration enables Claude Code instances to act as **orchestrators** for parallel AI development:

### Orchestration Workflow
```bash
# As orchestrator, dispatch multiple agents for parallel work
para_dispatch("api-endpoints", "Implement REST API with authentication")
para_dispatch("frontend-ui", "Create responsive user interface components")  
para_dispatch("database-schema", "Design and implement database schema")

# Monitor agent progress
para_list()  # Shows: api-endpoints (Active), frontend-ui (Active), database-schema (Active)

# Each agent works in isolation, then calls para_finish() when complete
# Orchestrator integrates results sequentially after agents finish
```

### Usage Examples

**Single Agent Session:**
```
para_start("feature-auth")  # Creates isolated worktree for development
# Work on feature...
para_finish("Implement user authentication")  # REQUIRED to complete
```

**Parallel Agent Dispatch:**
```
para_dispatch("agent1", "Task: Implement API endpoints. Must call para_finish() when done.")
para_dispatch("agent2", "Task: Create UI components. Must call para_finish() when done.")
para_dispatch("agent3", "Task: Database schema. Must call para_finish() when done.")
```

**Task File Integration:**
```
# Create task file: TASK_1_API.md with complete requirements
para_dispatch("api-agent", "See TASK_1_API.md for requirements", {"file": "TASK_1_API.md"})
```

**Orchestrator Monitoring:**
```
para_list()  # Monitor all active agent sessions
para_config_show()  # Check configuration for coordination
# Integration happens after agents finish their tasks
```

## Security Considerations

- MCP servers run with your user permissions
- Review para commands before agent execution
- Use in trusted environments only
- Monitor agent activities through para session logs

## Troubleshooting

### MCP Server Not Found
```bash
# Verify installation
which para-mcp-server

# Reinstall if missing
just install
```

### Claude Code Integration Issues
```bash
# Remove and re-add server
claude mcp remove para-server
claude mcp add para-server para-mcp-server
```

### VSCode/Cursor Configuration
```bash
# Verify MCP configuration file
cat .cursor/mcp.json

# Check Claude Desktop config (if using)
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

### Permission Issues
```bash
# Ensure binaries are executable
chmod +x ~/.local/bin/para-mcp-server

# Check PATH includes ~/.local/bin
echo $PATH
```

## Advanced Configuration

### Custom Environment Variables
```json
{
  "mcpServers": {
    "para": {
      "command": "para-mcp-server",
      "env": {
        "PARA_CONFIG_DIR": "/custom/config/path"
      }
    }
  }
}
```

### Debugging MCP Communication
```bash
# Test MCP server directly
echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2024-11-05","capabilities":{"resources":true,"tools":true},"client_info":{"name":"test","version":"1.0"}},"id":1}' | para-mcp-server
```

## Integration with Para Workflows

### Parallel Development with AI Agents
```bash
# Create task files for agents
para dispatch agent1 --file TASK_1_API.md -d
para dispatch agent2 --file TASK_2_UI.md -d  
para dispatch agent3 --file TASK_3_DB.md -d

# Agents can use MCP tools to:
# - Check their session status
# - Finish their tasks automatically
# - List other active sessions
# - Access para configuration
```

### Context-Aware Development
```bash
# Agents can read current session state
para://current-session -> Session details, git status, worktree path
para://available-sessions -> All session info for coordination
para://config -> Para configuration for context
```

## Best Practices

1. **Session Naming**: Use descriptive names for MCP-created sessions
2. **Task Isolation**: Keep agent tasks independent to avoid conflicts
3. **Regular Cleanup**: Use `para clean` to remove completed sessions
4. **Monitor Activity**: Check `para list` regularly when multiple agents are active
5. **Commit Messages**: Ensure agents provide meaningful commit messages

## Support

For MCP integration issues:
1. Check para logs: `para list --verbose`
2. Verify MCP server: `para-mcp-server --help`
3. Test editor integration: Follow editor-specific troubleshooting
4. Report issues: Include MCP server logs and editor configuration
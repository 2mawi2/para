# MCP Integration Guide

Para integrates seamlessly with Claude Code through the Model Context Protocol (MCP), providing native Para tools directly within Claude Code's interface. This eliminates the need for manual command execution and enables more efficient AI-assisted development workflows.

## Quick Setup

### Production Installation (Recommended)
```bash
# Install para (when available)
brew install 2mawi2/tap/para

# Navigate to your project
cd your-project

# One-time MCP setup
para mcp init --claude-code
```

### Development Installation
```bash
# Clone and build from source
git clone https://github.com/2mawi2/para.git
cd para
just install        # Installs para + MCP server to ~/.local/bin/

# Navigate to your project
cd your-project
para mcp init --claude-code
```

### Interactive Setup
```bash
para mcp init
# Choose your IDE from the interactive menu
```

**That's it!** Open Claude Code in your repo and Para tools will be available.

## Installation Scenarios

### Development Workflow (Repository Contributors)
When working on the Para repository itself:
```bash
git clone https://github.com/2mawi2/para.git
cd para
just install                    # Installs to ~/.local/bin
para mcp init --claude-code     # Uses TypeScript MCP server
```
**Result**: Uses the TypeScript MCP server from `mcp-server-ts/` for better debugging.

### Production Workflow (End Users)
For users installing Para to use in their projects:
```bash
brew install 2mawi2/tap/para    # When available
cd your-project
para mcp init --claude-code     # Uses TypeScript MCP server
```
**Result**: Uses the TypeScript MCP server installed by Homebrew (with Node.js wrapper).

### Manual Installation
Building and installing from source:
```bash
git clone https://github.com/2mawi2/para.git
cd para
just install                    # Builds and installs to ~/.local/bin
cd your-project
para mcp init --claude-code     # Uses installed TypeScript MCP server
```
**Result**: Uses the TypeScript MCP server from the built source.

## How Server Discovery Works

Para automatically finds the best available MCP server:
1. **Local TypeScript server**: `./mcp-server-ts/build/para-mcp-server.js` (development)
2. **Local installation**: `~/.local/bin/para-mcp-server` (manual install)
3. **Homebrew server**: `/opt/homebrew/bin/para-mcp-server` (Apple Silicon)
4. **Homebrew server**: `/usr/local/bin/para-mcp-server` (Intel Mac) 
5. **Linux Homebrew**: `/home/linuxbrew/.linuxbrew/bin/para-mcp-server`
6. **System PATH**: Fallback to `para-mcp-server` in PATH

## Generated Configuration

### Development (TypeScript Server)
```json
{
  "mcpServers": {
    "para": {
      "type": "stdio",
      "command": "node",
      "args": ["/path/to/repo/mcp-server-ts/build/para-mcp-server.js"]
    }
  }
}
```

### Production (Homebrew TypeScript Server)
```json
{
  "mcpServers": {
    "para": {
      "type": "stdio", 
      "command": "/opt/homebrew/bin/para-mcp-server",
      "args": []
    }
  }
}
```

**Important**: `.mcp.json` contains user-specific paths and should be added to `.gitignore`. Each team member should run `para mcp init --claude-code` in their local repo to generate their own config.

## IDE Support

### Claude Code (Recommended)
```bash
para mcp init --claude-code
```
- Project-scoped `.mcp.json` configuration
- Native tool integration with automatic discovery
- Verify with: `claude mcp list`

### Cursor
```bash
para mcp init --cursor
```
- Project-scoped `.mcp.json` configuration
- Supports MCP protocol

### VS Code with Roo Cline
```bash
para mcp init --vscode
```
- Project-scoped `.mcp.json` configuration
- Works with Roo Cline extension

## Available Para Tools

Once MCP integration is set up, Claude Code gains access to these Para tools:

### Core Session Management
- **`para_start`** - Create new isolated sessions with Git worktrees
  - `session_name` (required): Name for the new session
  
- **`para_finish`** - Complete sessions with automatic staging and commits
  - `commit_message` (required): Commit message for the changes

- **`para_list`** - List all active sessions with status and branch information

### Advanced Operations
- **`para_dispatch`** - AI-assisted session creation with prompts
  - `session_name` (required): Name for the new session
  - `task_description` (required): Task description for the AI agent

- **`para_recover`** - Recover and resume previous sessions
  - `session_name` (required): Name of the session to recover

- **`para_cancel`** - Cancel and delete sessions, removing worktrees and branches
  - `session_name` (required): Name of the session to cancel

- **`para_config_show`** - Display current Para configuration

### Available Resources

Claude Code can also read these Para resources for context:
- **`para://current-session`** - Information about the current para session
- **`para://config`** - Current para configuration

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
para mcp init --claude-code
# Error: No para MCP server found
```

**Solutions:**
- **Development**: `cd mcp-server-ts && npm install && npm run build`
- **Production**: `just install` or wait for Homebrew formula
- **Manual**: Ensure para is in your PATH with `which para`

### Claude Code Not Detecting Tools
1. Verify `.mcp.json` exists in your project root
2. Restart Claude Code completely
3. Run `claude mcp list` to verify server registration
4. Check that para binary is accessible from the configured path

### Server Path Issues
If you move your para installation:
```bash
rm .mcp.json
para mcp init --claude-code  # Regenerates with correct paths
```

### Permission Issues
```bash
# Ensure binaries are executable
chmod +x ~/.local/bin/para-mcp-server

# Check PATH includes ~/.local/bin
echo $PATH | grep -o ~/.local/bin
```

### TypeScript Server Issues (Development)
```bash
# Rebuild TypeScript server
cd mcp-server-ts
npm install
npm run build

# Verify it runs
node build/para-mcp-server.js --help
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

## Benefits Over Traditional Dispatch

| Traditional `para dispatch` | MCP Integration |
|----------------------------|-----------------|
| Requires command-line usage | Native Claude Code tools |
| Manual session management | Integrated workflow |
| Limited to dispatch command | Full Para toolset |
| Requires IDE switching | Seamless integration |

## Team Collaboration

### Setup for Teams

**Don't commit `.mcp.json`** - it contains user-specific paths. Instead:

1. **Add to `.gitignore`**:
```bash
echo ".mcp.json" >> .gitignore
git add .gitignore
git commit -m "Ignore user-specific MCP config"
```

2. **Document in README**:
```markdown
## MCP Setup
Run `para mcp init --claude-code` to enable Para tools in Claude Code.
```

3. **Each team member runs**:
```bash
para mcp init --claude-code
```

This generates the correct paths for their system while keeping the repo clean.

## Cross-Platform Support

The MCP integration works across all platforms:
- **macOS**: Supports both Intel and Apple Silicon Homebrew installations
- **Linux**: Supports Linuxbrew and manual installations  
- **Windows**: Supports manual installations

## Best Practices

1. **Commit Configuration**: Always commit `.mcp.json` for team sharing
2. **Descriptive Sessions**: Use clear session names for MCP-created sessions
3. **Task Isolation**: Keep agent tasks independent to avoid conflicts
4. **Regular Monitoring**: Use `para_list` to check active sessions
5. **Meaningful Commits**: Provide clear commit messages when finishing sessions

## What's Next

With MCP integration set up, you can:
1. Use Para tools directly from Claude Code's interface
2. Create isolated development sessions without leaving your IDE
3. Manage multiple parallel features efficiently
4. Leverage AI assistance with proper workspace isolation

The MCP integration makes Para's parallel development capabilities a natural part of your Claude Code workflow.
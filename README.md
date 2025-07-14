# Para - Parallel IDE Workflow Helper

Para enables developers to run multiple Claude Code sessions simultaneously. Each session operates in an isolated Git worktree with its own VS Code instance, allowing parallel development without merge conflicts.

## The Problem

When using Claude Code for development, you're limited to one task at a time. Switching between features means losing context and waiting for sequential completion.

## The Solution

Para orchestrates multiple Claude Code sessions in parallel. While one Claude works on authentication, another builds the API, and a third creates the UI - all simultaneously in separate VS Code windows.

## Simple Example

```bash
# Start multiple Claude Code sessions for different tasks
para start auth -p "implement user authentication"    # Opens VS Code with Claude working on auth
para start api -p "create REST API endpoints"        # Opens another VS Code with Claude on API

# Each Claude session:
# - Works in its own VS Code window
# - Has its own Git branch and isolated files
# - You can edit code, review diffs, and guide Claude
# - Full terminal access for running tests, etc.

# When Claude completes a task:
para finish "Add authentication"  # Creates feature branch locally for review
```

## Key Features
- **Parallel Execution**: Run multiple Claude Code instances concurrently in separate VS Code windows
- **Developer Control**: Direct access to each session's filesystem, terminal, and Git state
- **Isolation**: Git worktrees prevent conflicts between parallel development tasks
- **Monitoring**: Real-time dashboard showing session progress, test results, and blockers
- **Intervention**: Jump into any session to debug, guide Claude, or complete tasks manually

## Demo Videos

**Parallel Session Creation (0:03)**  
Shows orchestrator Claude creating and dispatching three parallel development tasks via MCP.

<video width="800" controls>
  <source src="docs/media/recording_starting_agents.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

**Real-time Monitoring Dashboard (0:04)**  
Demonstrates the Para monitor tracking multiple active sessions and their progress.

<video width="800" controls>
  <source src="docs/media/recording_para_monitor.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

## How Para Works

1. **Session Creation**: Each `para start` creates an isolated Git worktree with its own branch
2. **Claude Integration**: Claude Code launches in a new VS Code window with your specified task
3. **Parallel Execution**: Multiple sessions run independently without file conflicts
4. **Developer Access**: Full terminal, Git, and filesystem access in each VS Code window
5. **Monitoring**: Real-time dashboard tracks all active sessions via `para monitor`
6. **Completion**: `para finish` creates feature branches from completed work

## Installation

```bash
brew install 2mawi2/tap/para
```

For building from source, see the [Development Guide](docs/DEVELOPMENT.md).

## Getting Started

```bash
# Configure Claude Code as your IDE
para config

# Enable MCP integration in your project
para mcp init

# Start your first parallel session
para start backend -p "create REST API with Express"

# Start another session (opens in new VS Code window)
para start frontend -p "build React dashboard"

# Monitor all sessions
para monitor
```



## Orchestrating Parallel Development

### Using Claude as Your Development Orchestrator

Work with one Claude Code session to coordinate your entire project. Claude has built-in knowledge of Para commands through MCP integration, allowing it to:

- Break down complex features into parallel tasks
- Start new Claude sessions for each task
- Monitor progress across all sessions
- Help review and integrate completed work

### Example: Building a Web Application

```
Task Breakdown:
1. Database schema (must complete first)
2. API endpoints (depends on schema)
3. Frontend UI (can start with API)
4. Authentication (parallel with UI)
5. Integration tests (after all complete)

Parallel Execution:
- Session 1: Database → API endpoints
- Session 2: Frontend UI
- Session 3: Authentication
- Session 4: Tests (starts later)
```

### Automating Your Workflow

Create custom Claude slash commands to match your team's development process:

```bash
# Example commands in .claude/commands/
/breakdown   # Analyzes feature and creates parallel tasks
/review      # Reviews all completed branches
/integrate   # Suggests merge order

# Customize for your workflow:
- PR creation vs local branches
- Branch naming conventions
- Review requirements
- Integration strategy
```

The key insight: With multiple Claude sessions working in parallel, code review becomes the primary constraint. Automation helps manage this increased throughput.

## Security Considerations

### Autonomous Execution

When running Claude with `--dangerously-skip-permissions` (bypasses IDE permission prompts), use sandboxing for safety:

```bash
# macOS: Restrict file writes to session directory
para start -p "build feature" --dangerously-skip-permissions --sandbox

# macOS: Also restrict network access  
para start -p "refactor code" --dangerously-skip-permissions --sandbox-no-network

# All platforms: Full isolation with Docker
para start -p "risky task" --dangerously-skip-permissions --container
```

For detailed sandboxing configuration and security options, see the [Sandboxing Guide](docs/SANDBOXING.md).

### Important Notes

- **Code Review**: Always review generated code before merging
- **Sandboxing**: Recommended when using `--dangerously-skip-permissions`
- **Disclaimer**: Users are responsible for reviewing and validating all AI-generated code


## When to Use Para

Para is designed for developers who want to leverage multiple Claude Code sessions for faster development:

- **Feature Development**: Run parallel Claude sessions for frontend, backend, and tests
- **Large Refactoring**: Split refactoring across modules with multiple Claude instances  
- **Bug Fixing**: One Claude fixes bugs while another adds features
- **Experimentation**: You can ask claude to run multiple para sessions on the same plan file and keep the best result

Without Para, you're limited to one Claude Code session at a time. With Para, you can orchestrate an entire team of Claude instances.

## AI Integration

### MCP Integration

Para integrates with Claude Code through MCP (Model Context Protocol). The typical workflow is:

1. **Plan Development** - Work with Claude to create detailed implementation plans
2. **Save Plans** - Write these plans to .md files (wherever you prefer)
3. **Execute in Parallel** - Claude starts Para sessions from these plan files

Example workflow prompts:

**Planning Phase:**
- "Let's plan how to implement user authentication. Write a simple plan covering database, API, and frontend"
- "Break down this payment integration into independent tasks that can be developed in parallel"
- "Create a simple MVP implementation plan for this feature that avoids merge conflicts between components"

**Best Practice:** Start with simple, MVP-focused plans. Avoid overengineering in agent sessions - you can always iterate and expand later.

**Saving Plans** (you choose where):
- "Write this plan to `tasks/auth-implementation.md`"
- "Save each component plan to separate files in `planning/`" 
- "Create numbered task files: `TASK_1_database.md`, `TASK_2_api.md`"

**Executing Plans:**
- "Start a Para session from the auth implementation plan we just created"
- "Launch Para sessions for each of the task files in the planning directory"
- "Begin parallel development using the plans in `tasks/TASK_*.md`"

The key is that **you and Claude agree on the implementation details first**, then Para executes these well-defined plans in parallel. This constrains the degree of freedom each Claude Agent and reduces the probability of YAGNI and reward hacking.

Note: You can use any planning tool you prefer - local .md files, GitHub issues, or tools like [claude-task-master](https://github.com/eyaltoledano/claude-task-master). Para just needs access to read the plan files.

## Complete Example: Building a Todo App

Here's how the full workflow looks in practice:

1. **Plan with Claude**: "Let's build a todo app. Create a simple plan covering backend API, frontend UI, and database setup"

2. **Claude creates plan files**:
   ```
   tasks/todo-backend.md   - REST API with Express
   tasks/todo-frontend.md  - React UI components  
   tasks/todo-database.md  - PostgreSQL schema
   ```

3. **Start parallel sessions**: "Launch Para sessions for each of these task files"
   - Claude runs `para_start` for each plan file
   - 3 VS Code windows open, each with Claude working on a different component

4. **Monitor progress**: You run `para monitor` or just `para` to watch all sessions
   - See which sessions are coding, testing, or blocked
   - Real-time updates on progress and test results

5. **Intervene when needed**: Frontend session has styling issues
   - Jump into that VS Code window: `cd .para/worktrees/todo-frontend` (or resume in the para monitor)
   - Fix the CSS, guide Claude, run tests locally
   - Continue development together

6. **Complete and integrate**: Sessions finish their work
   - Each creates a feature branch: `para finish "Add todo backend"`
   - You review the 3 branches and merge them
   - Full todo app is ready

This workflow lets you build complex applications 3-5x faster while maintaining full control.

## Essential Commands

```bash
# Start new sessions
para start backend -p "create REST API"           # New AI session  
para start --file tasks/auth-plan.md              # From plan file
para start frontend                               # Manual session

# Monitor and control
para monitor                                      # Real-time dashboard
para list                                         # Show active sessions  
para finish "Add user authentication"            # Complete session
para config                                      # Setup IDE
```

For complete command reference, see [CLI Documentation](docs/CLI_REFERENCE.md).

## Example Workflows

### Orchestrated Development (Recommended)
```bash
# 1. Plan with Claude
"Break down user authentication into parallel tasks"
# Claude creates: tasks/auth-api.md, tasks/auth-ui.md, tasks/auth-tests.md

# 2. Execute plan  
"Start Para sessions for each auth task file"
# 3 VS Code windows open with Claude working

# 3. Monitor and guide
para monitor              # Watch progress
# Jump into sessions as needed to help
```

### Manual Session Management  
```bash
# Start individual Claude sessions directly
para start auth -p "implement JWT authentication"
para start ui -p "create login form"
para start tests -p "add auth unit tests"

# Or start manual sessions (no AI prompt)
para start feature-payments    # You work directly in VS Code
para start bugfix-login       # Manual debugging session
```

Both approaches use isolated Git worktrees to prevent conflicts.

## Advanced Features

- **Custom Branch Names**: `para finish "message" --branch custom-name`
- **Session Recovery**: Auto-backup of last 3 cancelled sessions  
- **Resume with Context**: Add instructions when resuming sessions
- **IDE Integration**: Works with Claude Code, VS Code, Cursor, and others

For detailed configuration and advanced usage, see the [Documentation](#documentation) section.


## Documentation

### Getting Started
- **[CLI Reference](docs/CLI_REFERENCE.md)** - Complete command documentation
- **[Shell Completion Guide](docs/SHELL_COMPLETION.md)** - Shell completion setup and configuration
- **[Detailed Configuration](docs/DETAILED_CONFIGURATION.md)** - Advanced IDE setup and environment variables

### AI Integration  
- **[MCP Integration](docs/MCP_INTEGRATION.md)** - Complete guide to Claude Code MCP integration
- **[MCP Orchestration](docs/MCP_ORCHESTRATION.md)** - AI orchestration patterns and workflows
- **[Sample Instructions](docs/SAMPLE_PARA_INSTRUCTIONS.md)** - Example CLAUDE.md for CLI-based workflows

### Security & Advanced
- **[Sandboxing Guide](docs/SANDBOXING.md)** - Complete sandboxing configuration and security options
- **[Workflow Guide](docs/WORKFLOW.md)** - Visual diagrams of Para workflows and state transitions
- **[IDE Behavior](docs/IDE_BEHAVIOR.md)** - IDE window management during integration and conflicts

### Support
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions  
- **[Development Guide](docs/DEVELOPMENT.md)** - Contributing, architecture, testing

## Requirements

- Git 2.5+ (for worktree support)
- Your preferred IDE with CLI support

## Getting Productive

1. **Install**: `brew install 2mawi2/tap/para`
2. **Configure**: `para config` (sets up your IDE)
3. **Enable MCP**: `para mcp init` (for Claude Code integration)
4. **Start building**: Use the planning workflow above

For shell completion: `para init`

## Advanced Configuration

Para supports environment variables, CI automation, and detailed customization. See the documentation links below for complete configuration options.

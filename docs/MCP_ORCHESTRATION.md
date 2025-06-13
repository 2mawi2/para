# Para MCP Orchestration Guide

This guide explains how Para's MCP (Model Context Protocol) integration enables AI orchestration for parallel development workflows.

## Overview

Para's MCP tools allow an orchestrator agent (like Claude) to manage multiple AI agents working in parallel on different features. Each agent works in an isolated Git worktree, preventing conflicts and enabling true parallel development.

## MCP vs CLI Tools

### MCP Tools (for Orchestrator)
- **para_dispatch** - Primary tool for launching agents
- **para_integrate** - Manual integration if needed
- **para_list** - Check status
- **para_start** - Manual sessions with user
- **para_finish** - Rarely used
- **para_cancel** - Cleanup abandoned work

### CLI Commands (for Dispatched Agents)
- **para finish** - Creates branch for review
- **para integrate** - Auto-merges to main branch
- **para list** - Check their own status

## Orchestration Workflow

### 1. Task Creation
Tasks are created in the `tasks/` directory by default:
```
tasks/
├── TASK_1_api_spec.md
├── TASK_2_frontend.md
└── TASK_3_backend.md
```

### 2. Task Writing Guidelines
- **Keep it simple** - Avoid overengineering
- **State WHAT not HOW** - Let agents choose implementation
- **Clear boundaries** - Prevent file conflicts
- **Include workflow** - End with integration command

### 3. Parallelization Strategy
```
Sequential:
1. API specification (foundation)
2. Then parallel: Frontend + Backend + Tests

Parallel:
- Independent features (auth, payments, email)
- Different layers using same interface
```

### 4. Dispatch and Integration
```bash
# Orchestrator dispatches agents
para_dispatch(session_name="api-spec", file="tasks/TASK_1_api_spec.md")

# Agent completes and integrates automatically
para integrate "Add API specification"

# Orchestrator continues with user on next tasks
# No monitoring needed - agents handle their own integration
```

## Example Task File

`tasks/TASK_1_user_auth.md`:
```markdown
# Task: User Authentication

Implement user authentication system with:
- Email/password login
- JWT token generation
- Password hashing
- User registration endpoint

Requirements:
- Use existing database connection
- Follow REST conventions
- Include input validation

When done: para integrate "Add user authentication"
```

## MCP Tool Descriptions

The MCP tools include comprehensive documentation in their descriptions. Key points:

1. **para_dispatch** explains:
   - Parallelization strategy
   - Task complexity guidelines
   - Workflow options
   - Example task format

2. **Automatic Integration**:
   - Agents use `para integrate` to auto-merge
   - Conflicts create branches for manual resolution
   - Orchestrator doesn't need to monitor

3. **Task Organization**:
   - Default `tasks/` directory
   - Numbered task files for clarity
   - Clear workflow instructions

## Benefits

1. **True Parallelism**: Multiple agents work simultaneously
2. **Conflict Prevention**: Isolated worktrees prevent merge issues
3. **Automatic Integration**: Agents merge their own work
4. **Focus on Value**: Orchestrator works with user, not managing Git
5. **Self-Documenting**: MCP tools contain workflow documentation

## Configuration

Users can customize the workflow in their `CLAUDE.md`:
- Choose `para finish` for manual review workflow
- Choose `para integrate` for automatic integration
- Add `-d` flag preferences for automation

The MCP tools adapt to these preferences, making the system flexible for different team workflows.
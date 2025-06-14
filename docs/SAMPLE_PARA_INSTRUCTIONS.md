# Sample Para Instructions for CLI-Based Workflow

This is a sample instructions file for using Para with CLI commands directly (without MCP integration). Add this to your project's `CLAUDE.md` or agent instructions.

## Para Workflow Instructions

### Overview
Para enables parallel development using Git worktrees. Each task runs in an isolated environment, preventing conflicts.

### Core Commands

#### Starting Work
```bash
# For simple tasks (short, natural language only)
para dispatch "task-name" "Implement user login page"

# For complex tasks or special characters (RECOMMENDED)
para dispatch "task-name" --file tasks/TASK_1_feature.md

# Skip IDE permissions (for automation)
para dispatch "task-name" --file tasks/TASK_1_feature.md -d
```

**Important**: Prefer task files over inline descriptions when:
- Task contains code snippets, JSON, or special characters
- Task description is longer than a single sentence
- Task includes technical specifications or formatting

#### Completing Work
```bash
# Create branch for manual review
para finish "Implement feature X"
```

### Task Organization

Store task files in `tasks/` directory:
```
tasks/
├── TASK_1_api_definition.md
├── TASK_2_frontend_ui.md
└── TASK_3_backend_api.md
```

### Task Writing Guidelines

1. **Keep tasks simple** - Avoid overengineering and YAGNI
2. **State requirements clearly** - What needs to be done, not how
3. **Let agents choose implementation** - Don't over-constrain
4. **Include workflow command** - End with integration instruction

### Example Task File

`tasks/TASK_1_user_auth.md`:
```markdown
# Task: User Authentication System

Implement a user authentication system with the following requirements:

- Email and password based login
- Secure password storage (hashed)
- JWT token generation for sessions
- REST API endpoints for login and register
- Basic input validation

Use the existing database configuration.
Follow project conventions for API responses.

When complete, run: para finish "Add user authentication system"
```

### Parallelization Strategy

**Sequential Tasks** (must complete in order):
1. API specifications / interfaces
2. Database schemas
3. Shared utilities

**Parallel Tasks** (can run simultaneously):
- Frontend implementation
- Backend implementation  
- Test suites
- Documentation

**Avoid Conflicts**:
- Don't assign multiple agents to modify the same files
- Create clear boundaries between tasks
- Use interfaces/contracts to coordinate

### Integration Workflow

**Manual Review**:
```bash
para finish "Your commit message"
```
- Creates branch `para/session-name`
- Requires manual merge after review

### Best Practices

1. **Foundation First**: Complete API specs before implementations
2. **Clear Boundaries**: Each task should have distinct files
3. **Atomic Tasks**: Tasks should be completable independently
4. **Test Everything**: Include test requirements in tasks
5. **Document Decisions**: Agents should document their choices

### Workflow Preferences

Specify your team's preference at the beginning of tasks:

**For branch creation and manual review:**
```
When complete: para finish "commit message"
```

**For manual work (no auto-commit):**
```
Do not run any para commands on completion.
```

### Example Multi-Agent Workflow

1. **Orchestrator creates tasks**:
   ```bash
   # Create API specification first
   echo "Define REST API..." > tasks/TASK_1_api_spec.md
   
   # Create implementation tasks
   echo "Implement frontend..." > tasks/TASK_2_frontend.md
   echo "Implement backend..." > tasks/TASK_3_backend.md
   ```

2. **Dispatch sequential task**:
   ```bash
   para dispatch api-spec --file tasks/TASK_1_api_spec.md -d
   # Wait for completion
   ```

3. **Dispatch parallel tasks**:
   ```bash
   para dispatch frontend --file tasks/TASK_2_frontend.md -d
   para dispatch backend --file tasks/TASK_3_backend.md -d
   ```

4. **Agents work independently** and run:
   ```bash
   para finish "Add API endpoints"
   para finish "Add user interface"
   ```

5. **Manual integration** happens after review of each branch

### Troubleshooting

- **Merge conflicts**: Para creates a branch, resolve manually
- **Check status**: `para list` shows all active sessions
- **Cleanup**: `para cancel session-name` removes abandoned work
- **Recovery**: `para recover session-name` restores cancelled sessions
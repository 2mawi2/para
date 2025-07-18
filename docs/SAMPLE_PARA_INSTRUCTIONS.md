# Sample Para Instructions for CLI-Based Workflow

This is a sample instructions file for using Para with CLI commands directly (without MCP integration). Add this to your project's `CLAUDE.md` or agent instructions.

## Para Workflow Instructions

### Overview
Para enables parallel development using Git worktrees. Each task runs in an isolated environment, preventing conflicts.

### Core Commands

#### Starting Work
```bash
# For simple tasks (short, natural language only)
para start -p "Implement user login page"

# For complex tasks or special characters (RECOMMENDED)
para start --file tasks/TASK_1_feature.md

# Skip IDE permissions (for automation)
para start --file tasks/TASK_1_feature.md --dangerously-skip-permissions
```

**Important**: Prefer task files over inline descriptions when:
- Task contains code snippets, JSON, or special characters
- Task description is longer than a single sentence
- Task includes technical specifications or formatting

#### Completing Work
```bash
# Create branch for manual review (default naming)
para finish "Implement feature X"

# Create branch with custom name
para finish "Implement feature X" --branch feature/auth-system

# Create branch with custom name (short flag)
para finish "Implement feature X" -b bugfix/login-issue
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

When complete, run: para finish "Add user authentication system" --branch feature/auth
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
# Default branch naming
para finish "Your commit message"
# Creates branch `para/session-name`

# Custom branch naming
para finish "Your commit message" --branch feature/custom-name
# Creates branch `feature/custom-name`
```
- Requires manual merge after review
- Custom branch names override default `para/session-name` pattern

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
# Default branch name
When complete: para finish "commit message"

# Custom branch name
When complete: para finish "commit message" --branch feature/my-feature
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

2. **Start sequential task**:
   ```bash
   para start --file tasks/TASK_1_api_spec.md --dangerously-skip-permissions
   # Wait for completion
   ```

3. **Start parallel tasks**:
   ```bash
   para start --file tasks/TASK_2_frontend.md --dangerously-skip-permissions
   para start --file tasks/TASK_3_backend.md --dangerously-skip-permissions
   ```

4. **Agents work independently** and run:
   ```bash
   para finish "Add API endpoints" --branch feature/api-v1
   para finish "Add user interface" --branch feature/ui-components
   ```

5. **Manual integration** happens after review of each branch

### Troubleshooting

- **Merge conflicts**: Para creates a branch, resolve manually
- **Check status**: `para list` shows all active sessions
- **Cleanup**: `para cancel session-name` removes abandoned work
- **Recovery**: `para recover session-name` restores cancelled sessions
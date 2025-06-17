# Implement Multi-Dispatch CLI Command

## Objective
Add a new `multi-dispatch` command to Para that creates multiple git worktrees, each running Claude Code in split terminals within a single VS Code window.

## Requirements

### 1. Update CLI Parser (`src/cli/parser.rs`)
- Add new `MultiDispatch` variant to the `Commands` enum with aliases `["md", "multi"]`
- Create `MultiDispatchArgs` struct with:
  - `sessions: Vec<SessionDefinition>` (required) - Session definitions
  - `dangerously_skip_permissions: bool` - Skip permission checks flag
- Create `SessionDefinition` struct with:
  - `name: Option<String>` - Optional session name
  - `prompt: Option<String>` - Optional prompt text
- Implement `FromStr` for `SessionDefinition` to parse "name:prompt" format

### 2. Create New Command Module (`src/cli/commands/multi_dispatch.rs`)
- Import necessary dependencies similar to `dispatch.rs`
- Create `execute(config: Config, args: MultiDispatchArgs) -> Result<()>` function
- Validate Claude Code IDE requirement (reuse logic from dispatch)
- Process each session request:
  - Generate unique names for unnamed sessions
  - Validate session names
  - Create worktrees in `.para/worktrees/` directory
  - Save session states
  - Write prompt files if provided
- Create workspace configuration in `.para-workspaces/` directory
- Use IdeManager to setup and launch multi-worktree workspace
- Print summary of created sessions

### 3. Update Command Module (`src/cli/commands/mod.rs`)
- Add `pub mod multi_dispatch;` to expose the new module

### 4. Update Main Dispatcher
- Add handling for `Commands::MultiDispatch(args)` in the main match statement
- Call `commands::multi_dispatch::execute(config, args)?`

## Usage Examples
The command should support:
```bash
# Auto-generated names
para multi-dispatch "implement auth" "create UI" "setup database"

# Named sessions
para multi-dispatch "auth:implement authentication" "ui:create dashboard"

# Mixed
para multi-dispatch "auth:implement login" "create API endpoints"

# With permission skip
para multi-dispatch -S "feature1:task1" "feature2:task2"
```

## Testing
- Add unit tests for `SessionDefinition::from_str` parsing
- Test validation of Claude Code IDE requirement
- Test session name generation and validation
- Ensure proper error handling for duplicate sessions

## Notes
- Reuse existing utility functions from `dispatch.rs` where applicable
- Follow the same error handling patterns as existing commands
- The actual IDE integration will be handled by the second agent

When done: para finish "Add multi-dispatch CLI command structure"
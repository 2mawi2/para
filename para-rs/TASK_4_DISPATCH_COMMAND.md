# Task 4: Implement Dispatch Command

## Objective
Implement the `para dispatch` command that creates a new session and launches Claude Code with an initial prompt.

## Key Requirements

### Command Behavior
- `para dispatch "prompt text"` - Create session with inline prompt
- `para dispatch session-name "prompt text"` - Create named session with prompt
- `para dispatch --file prompt.txt` - Create session with prompt from file
- `para dispatch session-name --file prompt.md` - Named session with file prompt
- `para dispatch "prompt" --dangerously-skip-permissions` - Skip IDE permission checks

### Core Functionality
1. **Session Creation**: Same as start command (create worktree, branch, state)
2. **Prompt Handling**: Support inline prompts and file-based prompts
3. **Claude Code Integration**: Launch Claude Code with initial prompt
4. **IDE Validation**: Ensure dispatch only works with Claude Code IDE
5. **File Input**: Read and validate prompt files
6. **Context Setup**: Prepare session with appropriate context

### Implementation Files to Modify
- `src/cli/commands/dispatch.rs` - Main command implementation
- Integrate with existing `config`, `core::git`, and `utils` modules

### Expected Integration Points
- Use `ConfigManager` to validate IDE is Claude Code
- Reuse session creation logic from start command
- Use file system utilities for reading prompt files
- Use `GitService` for worktree and branch creation
- Store prompt in session state for reference

### Claude Code Launch Logic
1. Validate IDE configuration is Claude Code
2. Handle wrapper mode (Claude Code inside VS Code/Cursor)
3. Build appropriate command line for Claude Code launch
4. Pass initial prompt as argument or through stdin
5. Handle IDE launch failures gracefully

### Prompt File Support
- Support common formats (.txt, .md, .prompt)
- Validate file exists and is readable
- Handle large prompt files appropriately
- Preserve prompt content in session state

### Success Criteria
- Command creates session like start command
- Reads prompts from files correctly
- Launches Claude Code with initial prompt
- Only works when IDE is configured as Claude Code
- Provides clear error when wrong IDE configured
- Handles both inline and file-based prompts
- Compatible with wrapper mode configurations

### Error Handling
- Handle case when IDE is not Claude Code
- Handle case when prompt file doesn't exist or isn't readable
- Handle case when Claude Code launch fails
- Handle case when prompt is empty or invalid
- Provide helpful error messages with suggested fixes
- Handle permission issues with IDE launch

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for dispatch command logic
- **Integration Tests**: Test end-to-end dispatch command functionality
- **Legacy Tests**: Ensure ALL legacy dispatch tests pass (36+ tests currently passing)
- **File Handling Tests**: Test prompt file reading and validation
- **IDE Integration Tests**: Test Claude Code launch scenarios
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors

## Completion Process
1. Implement the dispatch command functionality
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Run legacy tests to ensure compatibility (especially test_para_argument_parsing.bats)
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement dispatch command"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.
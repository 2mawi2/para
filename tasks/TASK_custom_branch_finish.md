# Task: Add Custom Branch Name Support to Para Finish

## Overview
Add the ability to specify custom branch names when using `para finish`, allowing users to override the default `para/session-name` pattern without changing the global configuration.

## Requirements

### CLI Interface Changes
- Add `--branch` or `-b` flag to `para finish` command
- Support custom branch name specification: `para finish "commit message" --branch custom-branch-name`
- Maintain backward compatibility with existing `para finish "message"` usage

### Edge Case Handling
- Check if custom branch name already exists
- If branch exists, provide clear error message with suggestions (append suffix, use different name, etc.)
- Validate branch name format (Git-compatible naming rules)
- Handle special characters and spaces appropriately

### Code Changes Required
1. Update `finish` command CLI argument parsing in `src/cli/commands/finish.rs`
2. Modify finish logic to use custom branch name when provided
3. Update branch creation/validation logic in core Git operations
4. Ensure MCP integration handles custom branch names properly

### Documentation Updates
1. Update help text and CLI documentation for `para finish --help`
2. Update `docs/SAMPLE_PARA_INSTRUCTIONS.md` to include custom branch examples
3. Update any MCP prompting documentation to mention custom branch option
4. Add examples showing both default and custom branch workflows

### MCP Integration
- Ensure MCP tools properly expose the custom branch functionality
- Update MCP API descriptions to mention the branch option
- Test that dispatched agents can use custom branch names

### Testing
- Add unit tests for branch name validation
- Test edge cases (existing branches, invalid names)
- Test integration with existing finish workflow
- Verify MCP functionality works with custom branches

### Example Usage
```bash
# Default behavior (unchanged)
para finish "Add user authentication"

# Custom branch name
para finish "Add user authentication" --branch feature/auth-system

# Short flag
para finish "Add user authentication" -b bugfix/login-issue
```

### Implementation Notes
- Follow existing error handling patterns using `anyhow::Result`
- Use existing Git utilities and branch creation patterns
- Maintain consistency with other para commands
- Ensure proper cleanup if branch creation fails

When complete, run: para finish "Add custom branch name support to finish command"
# Claude Session Continuation Demo

## Feature Overview

Para now integrates with Claude's session continuation feature, allowing you to resume conversations with full context when working on parallel sessions.

## How It Works

1. **Session Discovery**: When you run `para resume`, para checks for existing Claude sessions in `~/.claude/projects/`

2. **Session Mapping**: Claude stores sessions by sanitized project paths. For example:
   - Worktree path: `/Users/you/project/.para/worktrees/my-feature`
   - Claude path: `~/.claude/projects/-Users-you-project--para-worktrees-my-feature/`

3. **Session Continuation**: If a Claude session is found, para will use:
   ```bash
   claude -r "<session-id>" "your prompt"
   ```
   
   If no session is found, para falls back to:
   ```bash
   claude -c
   ```

## Usage Examples

### Resume with a follow-up prompt
```bash
para resume my-feature "continue implementing the authentication"
```

This will:
1. Find your para session
2. Look for existing Claude session
3. Launch Claude with conversation history and your prompt

### Resume without prompt
```bash
para resume my-feature
```

This will:
1. Resume the session
2. Continue with existing Claude conversation if found
3. Start fresh Claude session otherwise

## Benefits

- **Context Preservation**: Keep your entire conversation history when switching between features
- **Seamless Workflow**: No need to manually track Claude session IDs
- **Automatic Fallback**: Works even if no Claude session exists
- **Prompt Support**: Pass follow-up instructions directly

## Technical Details

The implementation:
- Reads Claude's session storage format (JSONL files)
- Finds the most recent session for a worktree
- Passes the session ID to Claude CLI
- Supports custom prompts with proper escaping
- Falls back gracefully when no session exists

This feature is particularly useful for:
- Long-running feature development
- Complex debugging sessions
- Iterative refinement of implementations
- Switching between multiple parallel features
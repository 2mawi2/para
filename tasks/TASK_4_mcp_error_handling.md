# Task: Fix MCP Server Error Handling for Interactive Commands

## Problem
The MCP server hangs indefinitely when para commands require user input (like confirmation prompts), instead of returning an error to the client. This causes the MCP tool to appear stuck with no feedback.

## Example Case
When running `para cancel` on a session with uncommitted changes:
- The CLI prompts: "Session has uncommitted changes. Are you sure? [y/N]:"
- The MCP server waits for input that never comes
- The MCP client hangs with no error message
- User has no idea what went wrong

## Root Cause
The MCP server executes para commands using `child.wait()` which blocks indefinitely when the command is waiting for stdin input.

## Solution

### 1. Add Non-Interactive Mode Detection
In the MCP server, set an environment variable to indicate non-interactive mode:
```typescript
// In para-mcp-server.ts
const env = {
  ...process.env,
  PARA_NON_INTERACTIVE: '1',
  // or
  CI: '1'  // Many CLIs respect this
};
```

### 2. Update Para Commands to Check for Non-Interactive Mode
Modify commands that use stdin to check for non-interactive mode:
```rust
// In cancel.rs and similar commands
fn is_non_interactive() -> bool {
    std::env::var("PARA_NON_INTERACTIVE").is_ok() || 
    std::env::var("CI").is_ok() ||
    !atty::is(atty::Stream::Stdin)
}

fn confirm_cancel_with_changes(session_name: &str) -> Result<()> {
    if is_non_interactive() {
        return Err(ParaError::invalid_args(
            "Cannot cancel session with uncommitted changes in non-interactive mode. \
             Commit or stash changes first, or run interactively."
        ));
    }
    
    // ... existing interactive prompt code
}
```

### 3. Add Timeout to MCP Server
Implement command timeout in the MCP server:
```typescript
// Set a reasonable timeout (e.g., 30 seconds)
const timeout = setTimeout(() => {
  child.kill();
  reject(new Error(`Command timed out after 30 seconds: ${args.join(' ')}`));
}, 30000);

child.on('exit', (code) => {
  clearTimeout(timeout);
  // ... rest of handling
});
```

### 4. Add Force Flags Where Appropriate
For some commands, add force flags to skip confirmations:
```rust
#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
    
    /// Force cancel without confirmation prompts
    #[arg(long, short = 'f')]
    pub force: bool,
}
```

## Implementation Steps

1. **Update MCP server** to set PARA_NON_INTERACTIVE environment variable
2. **Add timeout handling** in MCP server (30 second default)
3. **Update interactive commands** to detect non-interactive mode:
   - `cancel` - When there are uncommitted changes
   - `clean` - When confirming multiple session deletion  
   - `config reset` - When confirming reset
   - Any other commands with prompts
4. **Add force flags** where it makes sense (optional)
5. **Test all commands** through MCP to ensure none hang

## Testing

1. Create a session with uncommitted changes
2. Try to cancel it via MCP - should get clear error, not hang
3. Test other interactive commands through MCP
4. Verify timeout works if command genuinely hangs
5. Test that normal CLI usage still works with prompts

## Success Criteria

- MCP commands never hang waiting for input
- Clear error messages when interactive input is required
- Appropriate suggestions in error messages (e.g., "commit changes first")
- No impact on normal CLI interactive usage
- All MCP commands complete within 30 seconds or return error

When complete, run: para finish "Fix MCP server error handling for interactive commands"
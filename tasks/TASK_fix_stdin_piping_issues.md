# Fix stdin piping implementation issues

## Review Findings

The stdin piping implementation has a critical issue: the stdin detection using `atty::is(atty::Stream::Stdin)` is not working correctly. When testing with piped input, it still returns true (stdin is a TTY), causing dispatch to fail.

## Required Fixes

### 1. Fix stdin detection
The current approach using `!atty::is(atty::Stream::Stdin)` isn't detecting piped input properly. Consider these solutions:

**Option A: Update atty crate**
- Current version 0.2 is quite old
- Consider updating to latest atty or switching to `is-terminal` crate (the modern replacement)

**Option B: Use standard library approach**
```rust
use std::io::IsTerminal;

if !std::io::stdin().is_terminal() {
    // Handle piped input
}
```

**Option C: Check for available data**
- Try reading from stdin with a timeout
- If data is available, it's piped

### 2. Add debugging
Add debug output to understand why detection fails:
```rust
eprintln!("DEBUG: Is stdin a tty? {}", atty::is(atty::Stream::Stdin));
eprintln!("DEBUG: Checking stdin for piped data...");
```

### 3. Enable integration tests
- Remove `#[ignore]` from integration tests in `tests/stdin_integration.rs`
- Ensure they run in CI pipeline
- Add more test cases for edge conditions

### 4. Test the fix
After implementing, test with:
```bash
# Basic piping
echo "test prompt" | para dispatch

# JSON piping  
jq '.tasks[] | select(.id == 1)' tasks.json | para dispatch

# Empty input
echo "" | para dispatch

# Large input
cat large_file.txt | para dispatch
```

### 5. Update documentation
- Add examples in help text showing piping usage
- Document any limitations or platform-specific behavior

## Implementation Priority
1. First fix the stdin detection issue (try Option B with std::io::IsTerminal if Rust version supports it)
2. Add debug logging to verify the fix
3. Enable and run integration tests
4. Update documentation

When done: para finish "Fix stdin piping detection and testing"
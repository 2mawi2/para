# Add stdin piping support to para dispatch

## Goal
Enable piping input directly to `para dispatch` without creating intermediate files, supporting workflows like:
```bash
jq '.tasks[] | select(.id == 1)' tasks.json | para dispatch
```

## Implementation Plan

### 1. Update DispatchArgs to support stdin
- Modify `resolve_prompt_and_session()` in `src/cli/commands/dispatch.rs`
- Add stdin detection when no other input is provided
- Use existing `atty::is(atty::Stream::Stdin)` pattern

### 2. Implementation approach
```rust
// Check if stdin has piped data and no other input provided
if self.name_or_prompt.is_none() && self.prompt.is_none() && self.file.is_none() 
   && !atty::is(atty::Stream::Stdin) {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    
    if buffer.trim().is_empty() {
        return Err(ParaError::invalid_args("Empty input from stdin"));
    }
    
    return Ok((None, buffer));
}
```

### 3. Edge cases to handle
- Empty stdin input
- Very large stdin input (consider 10MB limit)
- Binary data detection and rejection
- Error on mixing stdin with other input methods

### 4. Testing requirements
- Unit tests for stdin detection in `dispatch_args_tests`
- Test empty stdin, large input, binary data
- Integration test with actual piping scenarios
- Ensure backwards compatibility with existing usage

### 5. Documentation updates
- Update command help text in `parser.rs`
- Add piping examples to help output
- Consider adding to README examples

## Expected behavior
- `echo "prompt" | para dispatch` - works with default session name
- `cat task.txt | para dispatch` - reads file content via pipe
- `jq '.task' file.json | para dispatch` - processes JSON output
- All existing dispatch usage patterns remain unchanged

When done: para finish "Add stdin piping support to para dispatch"
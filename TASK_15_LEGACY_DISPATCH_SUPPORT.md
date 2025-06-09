# Task 15: Legacy Dispatch Support

## Objective
Implement full legacy dispatch command support in the Rust implementation to match the shell script behavior exactly, including file-based and inline prompt handling.

## Background
The current Rust dispatch implementation in `para-rs/src/cli/commands/dispatch.rs` is incomplete compared to the legacy shell implementation in `lib/para-commands.sh`. The legacy version has sophisticated argument parsing and file handling that needs to be replicated.

## Requirements

### 1. Command Line Interface Parity
Must support all legacy dispatch patterns:

```bash
# Inline prompt only
para dispatch "implement hello world function"

# Session name + inline prompt  
para dispatch my-feature "implement hello world function"

# File-based prompt (auto-detect)
para dispatch task-spec.md
para dispatch requirements.txt

# Session name + file-based prompt
para dispatch my-feature task-spec.md

# Explicit file flag
para dispatch --file task-spec.md
para dispatch -f requirements.txt
para dispatch my-feature --file task-spec.md

# With skip permissions flag
para dispatch "prompt" -d
para dispatch --dangerously-skip-permissions "prompt"
```

### 2. File Path Auto-Detection
Implement the `is_file_path()` logic from legacy:
- Contains `/` character
- Ends with `.txt`, `.md`, `.prompt` extensions  
- File exists at the path
- Auto-detect and treat as file input without requiring `--file` flag

### 3. Error Handling
Match legacy error messages and behavior:
- Validate Claude Code IDE requirement
- Handle missing files gracefully
- Provide helpful error messages for invalid argument combinations

### 4. Integration Points
- Use existing Rust config system
- Integrate with session creation logic
- Support wrapper mode functionality
- Maintain compatibility with IDE launching

## Implementation Details

### Files to Modify
1. `para-rs/src/cli/commands/dispatch.rs` - Main dispatch logic
2. `para-rs/src/cli/parser.rs` - Update argument parsing if needed
3. Add tests to verify parity with legacy behavior

### Key Functions to Implement
1. Enhanced `resolve_prompt_and_session()` method
2. Improved `is_likely_file_path()` function 
3. Better argument validation and error handling
4. File content reading with proper error messages

### Argument Parsing Logic
```rust
// Pseudo-code for argument resolution
match (name_or_prompt, explicit_prompt, file_flag) {
    // File flag provided - highest priority
    (_, _, Some(file)) => read_file_and_extract_session_name(),
    
    // Single argument - could be session+prompt, prompt, or file
    (Some(arg), None, None) => {
        if is_file_path(arg) {
            // Auto-detect file
            read_file_content(arg)
        } else {
            // Treat as inline prompt
            use_as_prompt(arg)
        }
    },
    
    // Two arguments - session name + (prompt or file)
    (Some(session), Some(prompt_or_file), None) => {
        if is_file_path(prompt_or_file) {
            read_file_with_session_name(session, prompt_or_file)
        } else {
            use_session_and_prompt(session, prompt_or_file)
        }
    },
    
    // Error cases
    _ => return appropriate_error()
}
```

### File Path Detection
Enhance the `is_likely_file_path()` function:
```rust
fn is_likely_file_path(input: &str) -> bool {
    // Check for path separators
    if input.contains('/') || input.contains('\\') {
        return true;
    }
    
    // Check for known extensions
    let extensions = [".txt", ".md", ".prompt", ".text"];
    if extensions.iter().any(|ext| input.ends_with(ext)) {
        return true;
    }
    
    // Check if file exists
    Path::new(input).exists()
}
```

## Testing Requirements
1. Create comprehensive tests matching legacy test patterns
2. Test all argument combinations from the legacy implementation
3. Verify file reading behavior matches shell script
4. Test error conditions and messages
5. Ensure integration with existing session creation works

## Validation Criteria
- [ ] All legacy dispatch command patterns work identically
- [ ] File auto-detection matches shell script behavior
- [ ] Error messages are consistent with legacy implementation  
- [ ] Integration with IDE launching works correctly
- [ ] Wrapper mode support maintained
- [ ] All tests pass (both new and existing)

## Notes
- Reference the legacy implementation in `lib/para-commands.sh` lines 102-161
- Pay special attention to the argument parsing logic in `parse_common_args()`
- Ensure the `create_new_session()` equivalent works with prompts
- Maintain compatibility with existing config and session management

## Completion
When complete, call `para finish "Implement legacy dispatch support with file and inline prompt handling"` to finish the task.

The agent should run all tests to ensure nothing is broken and verify the implementation matches the legacy behavior exactly.
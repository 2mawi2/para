# Task 13: Legacy Test Wrapper for Rust Binary Compatibility

## Objective
**DEPENDS ON: Task 12 (Clean Configuration System)**

Create a test wrapper script that allows existing legacy bats tests to run against the Rust binary by dynamically configuring the JSON config based on environment variables during test execution.

## Background
The existing bats tests use environment variables (like `IDE_NAME`, `IDE_CMD`, etc.) to configure para behavior. With Task 12 completed, the Rust implementation uses JSON-only configuration and ignores environment variables. 

We need to bridge this gap so legacy tests can run against the Rust binary during the transition period, without modifying the Rust implementation itself.

## Requirements

### 1. Test Wrapper Script
Create a wrapper script `tests/para-rust-wrapper.sh` that:

**Purpose:** Acts as a proxy between legacy tests and the Rust binary
**Location:** `/tests/para-rust-wrapper.sh` 
**Usage:** Legacy tests call this wrapper instead of calling the Rust binary directly

```bash
#!/usr/bin/env sh
# Wrapper script for running Rust para binary with legacy test environment variables

RUST_BINARY="${1:-./para-rs/target/debug/para}"
shift
COMMAND="$@"

# Save original config if it exists
CONFIG_FILE="$HOME/Library/Application Support/para/config.json"
BACKUP_CONFIG=""
if [ -f "$CONFIG_FILE" ]; then
    BACKUP_CONFIG=$(cat "$CONFIG_FILE")
fi

# Generate temporary config based on environment variables
generate_test_config() {
    # Map environment variables to JSON config
    IDE_NAME="${IDE_NAME:-cursor}"
    IDE_CMD="${IDE_CMD:-cursor}"
    # ... etc
}
```

### 2. Environment Variable Mapping
Map legacy shell environment variables to JSON config format:

**IDE Configuration:**
- `IDE_NAME` → `config.ide.name`
- `IDE_CMD` → `config.ide.command`
- `CURSOR_CMD` → `config.ide.command` (when IDE_NAME=cursor)
- `IDE_WRAPPER_ENABLED` → `config.ide.wrapper.enabled`
- `IDE_WRAPPER_NAME` → `config.ide.wrapper.name`  
- `IDE_WRAPPER_CMD` → `config.ide.wrapper.command`

**Directory Configuration:**
- `BRANCH_PREFIX` → `config.git.branch_prefix`
- `SUBTREES_DIR_NAME` → `config.directories.subtrees_dir`
- `STATE_DIR_NAME` → `config.directories.state_dir`

**Test Mode Variables:**
- `IDE_CMD="true"` → Test mode (handled by Rust implementation)
- `IDE_CMD="echo ..."` → Test stub mode (handled by Rust implementation)

### 3. Temporary Config Management
The wrapper must:

1. **Save existing config** before test execution
2. **Generate temporary config** based on environment variables
3. **Write temporary config** to the JSON config file
4. **Execute Rust binary** with the test command
5. **Restore original config** after test completion (even if test fails)

```bash
# Example wrapper logic
generate_and_write_test_config() {
    cat > "$CONFIG_FILE" << EOF
{
  "ide": {
    "name": "${IDE_NAME:-cursor}",
    "command": "${IDE_CMD:-cursor}",
    "user_data_dir": null,
    "wrapper": {
      "enabled": ${IDE_WRAPPER_ENABLED:-false},
      "name": "${IDE_WRAPPER_NAME:-}",
      "command": "${IDE_WRAPPER_CMD:-}"
    }
  },
  "directories": {
    "subtrees_dir": "${SUBTREES_DIR_NAME:-subtrees/pc}",
    "state_dir": "${STATE_DIR_NAME:-.para_state}"
  },
  "git": {
    "branch_prefix": "${BRANCH_PREFIX:-pc}",
    "auto_stage": true,
    "auto_commit": true
  },
  "session": {
    "default_name_format": "%Y%m%d-%H%M%S",
    "preserve_on_finish": true,
    "auto_cleanup_days": 30
  }
}
EOF
}
```

### 4. Error Handling and Cleanup
The wrapper must be robust:

```bash
# Trap to ensure cleanup even on failures
cleanup() {
    if [ -n "$BACKUP_CONFIG" ]; then
        echo "$BACKUP_CONFIG" > "$CONFIG_FILE"
    else
        rm -f "$CONFIG_FILE" 
    fi
}

trap cleanup EXIT INT TERM
```

### 5. Legacy Test Integration
Update the test runner to use the wrapper:

**In test files:** Replace direct Rust binary calls with wrapper calls
```bash
# Before (direct):
./para-rs/target/debug/para start test-session

# After (via wrapper):  
./tests/para-rust-wrapper.sh ./para-rs/target/debug/para start test-session
```

**Or create a test-specific binary alias:**
```bash
# In test setup
alias para="./tests/para-rust-wrapper.sh ./para-rs/target/debug/para"
```

## Files to Create/Modify

### New Files
1. **`tests/para-rust-wrapper.sh`**
   - Main wrapper script
   - Environment variable to JSON config mapping
   - Temporary config file management
   - Error handling and cleanup

### Modified Files  
2. **`tests/test_common.sh`** (if exists)
   - Add function to use wrapper when testing Rust binary
   - Add helper functions for test config management

3. **Individual test files** (as needed)
   - Update calls to use wrapper when testing Rust implementation
   - Ensure compatibility with both shell and Rust versions

## Implementation Strategy

### Phase 1: Core Wrapper Script
- Create basic wrapper that maps most common environment variables
- Focus on IDE configuration first (`IDE_NAME`, `IDE_CMD`)
- Test with simple bats test cases

### Phase 2: Complete Environment Variable Support
- Add all environment variable mappings
- Add robust error handling and cleanup
- Test with more complex scenarios

### Phase 3: Test Integration
- Update test runner to use wrapper for Rust binary tests
- Ensure test isolation (each test gets clean config)
- Verify both shell and Rust tests work

### Phase 4: Documentation and Cleanup
- Document the wrapper usage
- Create helper functions for test writers
- Plan for eventual removal when transition is complete

## Success Criteria

### Functional Requirements
1. **Legacy Tests Work**: Existing bats tests pass when run against Rust binary via wrapper
2. **Environment Variable Support**: All critical environment variables are properly mapped to JSON
3. **Test Isolation**: Each test runs with clean configuration based on its environment variables
4. **Config Preservation**: Original user config is preserved and restored after tests

### Quality Requirements
5. **Robust Cleanup**: Config is restored even if tests fail or are interrupted
6. **Error Handling**: Clear error messages when wrapper fails
7. **Performance**: Wrapper adds minimal overhead to test execution

### Compatibility Requirements
8. **Shell Tests Unaffected**: Existing shell-based tests continue to work unchanged
9. **Rust Binary Integration**: Wrapper works seamlessly with Rust binary
10. **Cross-Platform**: Works on macOS and Linux (primary development platforms)

## Testing the Wrapper

```bash
# Test basic IDE configuration
IDE_NAME=code IDE_CMD=code ./tests/para-rust-wrapper.sh ./para-rs/target/debug/para config show
# Should show VS Code configuration

# Test wrapper mode
IDE_NAME=claude IDE_WRAPPER_ENABLED=true IDE_WRAPPER_NAME=cursor \
  ./tests/para-rust-wrapper.sh ./para-rs/target/debug/para config show
# Should show Claude with Cursor wrapper enabled

# Test that config is restored
echo "Original config preserved after test"
./para-rs/target/debug/para config show
```

## Example Usage After Implementation

```bash
# Run legacy test with wrapper  
./tests/para-rust-wrapper.sh ./para-rs/target/debug/para start test-session

# Test with environment variables
IDE_NAME=code BRANCH_PREFIX=test \
  ./tests/para-rust-wrapper.sh ./para-rs/target/debug/para start test-session

# Integration with bats
bats tests/test_para_integration.bats  # Uses wrapper internally
```

## Dependencies and Timeline
- **Depends on**: Task 12 (Clean Configuration System) - provides JSON-only config foundation
- **Blocks**: Running legacy bats tests against Rust implementation
- **Priority**: Medium - needed for testing during transition period
- **Timeline**: Can be implemented in parallel with other Rust features

## Future Removal Plan
This wrapper is temporary infrastructure:
- Once all tests are converted to use JSON config directly
- Or once Rust implementation fully replaces shell version  
- The wrapper can be removed and tests simplified

This task enables smooth testing during the transition period without requiring changes to the Rust implementation itself.
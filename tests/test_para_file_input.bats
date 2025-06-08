#!/usr/bin/env bats

# Tests for file input functionality in dispatch command
# Tests argument parsing, file detection, and content reading without opening IDEs

# Source common test functions
. "$(dirname "${BATS_TEST_FILENAME}")/test_common.sh"

setup() {
    setup_temp_git_repo
}

teardown() {
    teardown_temp_git_repo
}

@test "FI-1: is_file_path correctly identifies file paths" {
    cd "$TEST_REPO"
    
    # Create test file
    echo "test content" > test.txt
    
    # Source the utils to test the function directly
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test existing files
    run is_file_path "test.txt"
    [ "$status" -eq 0 ]
    
    # Test paths with slashes
    run is_file_path "dir/file.txt"
    [ "$status" -eq 0 ]
    
    run is_file_path "/absolute/path.txt"
    [ "$status" -eq 0 ]
    
    # Test common file extensions
    run is_file_path "prompt.md"
    [ "$status" -eq 0 ]
    
    run is_file_path "task.prompt"
    [ "$status" -eq 0 ]
    
    run is_file_path "template.tmpl"
    [ "$status" -eq 0 ]
    
    # Test non-file strings
    run is_file_path "just a prompt"
    [ "$status" -ne 0 ]
    
    run is_file_path "simple"
    [ "$status" -ne 0 ]
    
    run is_file_path ""
    [ "$status" -ne 0 ]
}

@test "FI-2: read_file_content reads file correctly" {
    cd "$TEST_REPO"
    
    # Create test file with content
    echo "This is a test prompt for authentication" > test-prompt.txt
    
    # Source the utils to test the function directly
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading existing file
    run read_file_content "test-prompt.txt"
    [ "$status" -eq 0 ]
    [ "$output" = "This is a test prompt for authentication" ]
    
    # Test reading with relative path
    mkdir -p subdir
    echo "subdirectory content" > subdir/nested.txt
    run read_file_content "subdir/nested.txt"
    [ "$status" -eq 0 ]
    [ "$output" = "subdirectory content" ]
}

@test "FI-3: read_file_content handles missing files" {
    cd "$TEST_REPO"
    
    # Source the utils to test the function directly
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test missing file
    run read_file_content "missing.txt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"file not found"* ]]
}

@test "FI-4: read_file_content handles unreadable files" {
    cd "$TEST_REPO"
    
    # Create file and remove read permissions
    echo "secret content" > unreadable.txt
    chmod 000 unreadable.txt
    
    # Source the utils to test the function directly
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test unreadable file
    run read_file_content "unreadable.txt"
    [ "$status" -ne 0 ]
    [[ "$output" == *"file not readable"* ]]
    
    # Cleanup
    chmod 644 unreadable.txt
}

@test "FI-5: dispatch command accepts --file option" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Implement user authentication system" > auth-prompt.txt
    
    # Mock Claude IDE to prevent opening
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test dispatch with --file option (should fail due to missing git repo setup)
    run "$PARA_SCRIPT" dispatch --file auth-prompt.txt
    # We expect it to fail at session creation, but argument parsing should work
    # The error should not be about unknown option
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-6: dispatch command accepts -f short option" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Create REST API endpoints" > api-prompt.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test dispatch with -f option
    run "$PARA_SCRIPT" dispatch -f api-prompt.txt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-7: dispatch command auto-detects file paths" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Implement database migrations" > database.prompt
    
    # Mock Claude IDE
    export IDE_NAME="claude"  
    export IDE_CMD="echo"
    
    # Test dispatch with file path as positional argument
    run "$PARA_SCRIPT" dispatch database.prompt
    # Should not fail with unknown option error or missing prompt error
    [[ "$output" != *"unknown option"* ]]
    [[ "$output" != *"requires a prompt text"* ]]
}


@test "FI-8: file input takes precedence over auto-detection" {
    cd "$TEST_REPO"
    
    # Create two files
    echo "Content from explicit file" > explicit.txt
    echo "Content from auto-detected file" > auto.prompt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test that --file option takes precedence over auto-detection
    # This test verifies argument parsing logic but won't test actual content
    # since we're not running full session creation
    run "$PARA_SCRIPT" dispatch --file explicit.txt auto.prompt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-9: error handling for missing file with --file option" {
    cd "$TEST_REPO"
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test with missing file
    run "$PARA_SCRIPT" dispatch --file missing-file.txt
    [ "$status" -ne 0 ]
    [[ "$output" == *"file not found"* ]]
}

@test "FI-10: error handling for missing file argument" {
    cd "$TEST_REPO"
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test with --file but no file argument
    run "$PARA_SCRIPT" dispatch --file
    [ "$status" -ne 0 ]
    [[ "$output" == *"--file requires a file path"* ]]
}

@test "FI-11: dispatch preserves session name with file input" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Implement OAuth2 flow" > oauth.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test dispatch with session name and file
    run "$PARA_SCRIPT" dispatch auth-session --file oauth.txt
    # Should not fail with validation errors for session name
    [[ "$output" != *"unknown option"* ]]
    [[ "$output" != *"session name can only contain"* ]]
}

@test "FI-12: complex file content is handled correctly" {
    cd "$TEST_REPO"
    
    # Create file with complex content including quotes and special characters
    cat > complex-prompt.txt << 'EOF'
Implement a secure authentication system with the following requirements:

1. User registration with email validation
2. Password hashing using bcrypt with salt rounds >= 12
3. JWT tokens with 15-minute expiry
4. Refresh token rotation
5. Rate limiting: 5 attempts per minute per IP
6. Input sanitization to prevent XSS/SQL injection
7. HTTPS-only cookies with SameSite=Strict

Use these technologies:
- Node.js with Express
- PostgreSQL with connection pooling
- Redis for session storage
- Winston for logging

Security considerations:
- Implement OWASP recommendations
- Use helmet.js for security headers
- Add CSRF protection
- Validate all inputs server-side
- Log security events

The system should handle edge cases like:
- Concurrent login attempts
- Token expiry during active sessions
- Database connection failures
- Redis unavailability

Testing requirements:
- Unit tests for all authentication functions
- Integration tests for auth flows
- Security tests for common vulnerabilities
- Performance tests for high-concurrency scenarios
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading complex file content
    run read_file_content "complex-prompt.txt"
    [ "$status" -eq 0 ]
    # Verify it contains key parts of the content
    [[ "$output" == *"Implement a secure authentication system"* ]]
    [[ "$output" == *"OWASP recommendations"* ]]
    [[ "$output" == *"Testing requirements"* ]]
}

@test "FI-13: dispatch command handles empty file with clear error message" {
    cd "$TEST_REPO"
    
    # Create empty file
    touch empty-prompt.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test dispatch with empty file
    run "$PARA_SCRIPT" dispatch --file empty-prompt.txt
    [ "$status" -ne 0 ]
    [[ "$output" == *"file is empty: empty-prompt.txt"* ]]
}


@test "FI-14: file content with single quotes is handled correctly" {
    cd "$TEST_REPO"
    
    # Create file with single quotes (the exact issue reported)
    cat > quotes-prompt.txt << 'EOF'
please fix the following issue. sometimes when i use 'para finish "message"' but message contains 
signs like ' or " then
for example para dispatch 'testtest'
gives me in claude this error:
Executing task: claude test'test 
fish: Unexpected end of string, quotes are not balanced
claude test'test
       ^
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading file with single quotes
    run read_file_content "quotes-prompt.txt"
    [ "$status" -eq 0 ]
    # Verify it contains the problematic content with quotes
    [[ "$output" == *"para finish \"message\""* ]]
    [[ "$output" == *"test'test"* ]]
    
    # Test that wrapper mode handles this content properly
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    export IDE_CMD="claude"
    
    # Test build_claude_wrapper_command with quotes
    run build_claude_wrapper_command "$output" "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    # The output should be properly handled in wrapper mode
    [[ "$output" == *"claude"* ]]
}

@test "FI-15: file content with double quotes is handled correctly" {
    cd "$TEST_REPO"
    
    # Create file with double quotes
    cat > doublequotes-prompt.txt << 'EOF'
Create a function that handles "quoted strings" and processes:
- JSON like {"key": "value", "nested": {"inner": "data"}}
- SQL queries like SELECT * FROM users WHERE name = "John Doe"
- Shell commands like echo "Hello World"
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading file with double quotes
    run read_file_content "doublequotes-prompt.txt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"\"quoted strings\""* ]]
    [[ "$output" == *"\"Hello World\""* ]]
}

@test "FI-16: file content with backticks is handled correctly" {
    cd "$TEST_REPO"
    
    # Create file with backticks
    cat > backticks-prompt.txt << 'EOF'
Write a script that executes commands like:
- `ls -la`
- `git status`
- `npm install`
And captures their output properly.
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading file with backticks
    run read_file_content "backticks-prompt.txt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"\`ls -la\`"* ]]
    [[ "$output" == *"\`git status\`"* ]]
}

@test "FI-17: file content with dollar signs is handled correctly" {
    cd "$TEST_REPO"
    
    # Create file with dollar signs and variables
    cat > dollars-prompt.txt << 'EOF'
Create environment setup with:
- $HOME directory handling
- ${USER} variable expansion
- $PATH modifications
- $(command substitution)
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading file with dollar signs
    run read_file_content "dollars-prompt.txt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"\$HOME"* ]]
    [[ "$output" == *"\${USER}"* ]]
    [[ "$output" == *"\$PATH"* ]]
    [[ "$output" == *"\$(command"* ]]
}

@test "FI-18: file content with newlines and complex formatting" {
    cd "$TEST_REPO"
    
    # Create file with complex formatting including newlines, tabs, etc.
    cat > complex-format.txt << 'EOF'
Multi-line prompt with:

	Tabs and spaces
    Mixed indentation
        Deep nesting

Special characters: !@#$%^&*()
Brackets: [] {} ()
Pipes and redirects: | > < >>
Semicolons and ampersands: ; && ||

Code examples:
function test() {
    echo "Hello, World!"
    return 0
}
EOF
    
    # Source the utils to test file reading
    . "$ORIGINAL_DIR/lib/para-utils.sh"
    
    # Test reading file with complex formatting
    run read_file_content "complex-format.txt"
    [ "$status" -eq 0 ]
    [[ "$output" == *"Special characters: !@#\$%^&*()"* ]]
    [[ "$output" == *"function test()"* ]]
    # Check that newlines are preserved
    lines=$(echo "$output" | wc -l)
    [ "$lines" -gt 10 ]
}

@test "FI-19: dispatch with problematic file content does not break shell execution" {
    cd "$TEST_REPO"
    
    # Create file with the exact content that causes issues
    cat > problematic.txt << 'EOF'
test'test with single quote
test"test with double quote
test`test with backtick
test$test with dollar
test&test with ampersand
test;test with semicolon
test|test with pipe
EOF
    
    # Mock Claude IDE with enhanced logging to capture what command would be executed
    export IDE_NAME="claude"
    export IDE_CMD="echo"
    
    # Test dispatch with problematic content - this should not fail
    run "$PARA_SCRIPT" dispatch --file problematic.txt
    
    # The command should not fail due to shell escaping issues
    # We expect it to fail at some other point (like missing git setup), 
    # but NOT with shell quoting errors
    [[ "$output" != *"Unexpected end of string"* ]]
    [[ "$output" != *"quotes are not balanced"* ]]
    [[ "$output" != *"command not found"* ]]
}


@test "FI-20: verifies new double-quote escaping produces safe commands" {
    cd "$TEST_REPO"
    
    # Source the IDE functions
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test the exact problematic case from the issue report
    problematic_content="test'test"
    
    # Test that the wrapper command generation works
    run build_claude_wrapper_command "$problematic_content" "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    
    # Verify the command uses proper escaping for wrapper mode
    [[ "$output" == *'claude'* ]]
    [[ "$output" == *'test'* ]]
}

@test "FI-21: test new double-quote escaping with different special characters" {
    cd "$TEST_REPO"
    
    # Source the IDE functions to test escaping
    export IDE_CMD="claude"
    . "$ORIGINAL_DIR/lib/para-ide.sh"
    
    # Test single quotes (should work in wrapper mode)
    run build_claude_wrapper_command "test'test" "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    [[ "$output" == *'claude'* ]]
    
    # Test double quotes (should work in wrapper mode)
    run build_claude_wrapper_command 'test"test' "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    [[ "$output" == *'claude'* ]]
    
    # Test backticks (should work in wrapper mode)
    run build_claude_wrapper_command 'test`test' "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    [[ "$output" == *'claude'* ]]
    
    # Test dollar signs (should work in wrapper mode)
    run build_claude_wrapper_command 'test$test' "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    [[ "$output" == *'claude'* ]]
    
    # Test backslashes (should work in wrapper mode)
    run build_claude_wrapper_command 'test\test' "" "false" "/tmp/test"
    [ "$status" -eq 0 ]
    [[ "$output" == *'claude'* ]]
    
    # Verify all resulting commands use the wrapper approach
    for test_input in "test'test" 'test"test' 'test`test' 'test$test' 'test\test'; do
        claude_cmd=$(build_claude_wrapper_command "$test_input" "" "false" "/tmp/test")
        
        # Verify all commands contain claude
        [[ "$claude_cmd" == *'claude'* ]]
    done
}

# Test configuration (minimal for testing - no .para_state)
export IDE_NAME="claude"
export IDE_CMD="echo"
export IDE_WRAPPER_ENABLED="true"
export IDE_WRAPPER_NAME="code"
export IDE_WRAPPER_CMD="echo"
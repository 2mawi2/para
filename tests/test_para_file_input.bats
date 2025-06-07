#!/usr/bin/env bats

# Tests for file input functionality in dispatch and dispatch-multi commands
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
    export IDE_CMD="echo 'claude-mock'"
    
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
    export IDE_CMD="echo 'claude-mock'"
    
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
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch with file path as positional argument
    run "$PARA_SCRIPT" dispatch database.prompt
    # Should not fail with unknown option error or missing prompt error
    [[ "$output" != *"unknown option"* ]]
    [[ "$output" != *"requires a prompt text"* ]]
}

@test "FI-8: dispatch-multi command accepts --file option" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Compare three authentication approaches" > multi-auth.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch-multi with --file option
    run "$PARA_SCRIPT" dispatch-multi 2 --file multi-auth.txt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-9: dispatch-multi command accepts -f short option" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Implement three different caching strategies" > cache-strategies.prompt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch-multi with -f option
    run "$PARA_SCRIPT" dispatch-multi 3 -f cache-strategies.prompt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-10: dispatch-multi command auto-detects file paths" {
    cd "$TEST_REPO"
    
    # Create prompt file with common extension
    echo "Design microservices architecture" > microservices.md
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch-multi with file as positional argument
    run "$PARA_SCRIPT" dispatch-multi 2 microservices.md
    # Should not fail with unknown option or missing prompt error
    [[ "$output" != *"unknown option"* ]]
    [[ "$output" != *"requires a prompt text"* ]]
}

@test "FI-11: dispatch-multi with --group and --file works together" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Evaluate different frontend frameworks" > frontend-eval.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch-multi with both --group and --file
    run "$PARA_SCRIPT" dispatch-multi 3 --group frontend --file frontend-eval.txt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-12: file input takes precedence over auto-detection" {
    cd "$TEST_REPO"
    
    # Create two files
    echo "Content from explicit file" > explicit.txt
    echo "Content from auto-detected file" > auto.prompt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test that --file option takes precedence over auto-detection
    # This test verifies argument parsing logic but won't test actual content
    # since we're not running full session creation
    run "$PARA_SCRIPT" dispatch --file explicit.txt auto.prompt
    # Should not fail with unknown option error
    [[ "$output" != *"unknown option"* ]]
}

@test "FI-13: error handling for missing file with --file option" {
    cd "$TEST_REPO"
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test with missing file
    run "$PARA_SCRIPT" dispatch --file missing-file.txt
    [ "$status" -ne 0 ]
    [[ "$output" == *"file not found"* ]]
}

@test "FI-14: error handling for missing file argument" {
    cd "$TEST_REPO"
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test with --file but no file argument
    run "$PARA_SCRIPT" dispatch --file
    [ "$status" -ne 0 ]
    [[ "$output" == *"--file requires a file path"* ]]
}

@test "FI-15: dispatch preserves session name with file input" {
    cd "$TEST_REPO"
    
    # Create prompt file
    echo "Implement OAuth2 flow" > oauth.txt
    
    # Mock Claude IDE
    export IDE_NAME="claude"
    export IDE_CMD="echo 'claude-mock'"
    
    # Test dispatch with session name and file
    run "$PARA_SCRIPT" dispatch auth-session --file oauth.txt
    # Should not fail with validation errors for session name
    [[ "$output" != *"unknown option"* ]]
    [[ "$output" != *"session name can only contain"* ]]
}

@test "FI-16: complex file content is handled correctly" {
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
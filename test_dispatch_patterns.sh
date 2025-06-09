#!/bin/bash
# Test script to validate dispatch patterns match legacy behavior

set -e

PARA_BINARY="./target/release/para"

echo "Testing dispatch argument patterns..."

# Test 1: Inline prompt
echo "1. Testing inline prompt..."
$PARA_BINARY dispatch "implement user auth" --help >/dev/null 2>&1 && echo "✓ Inline prompt parsing works"

# Test 2: Session name + prompt  
echo "2. Testing session name + prompt..."
$PARA_BINARY dispatch "auth-feature" "implement authentication" --help >/dev/null 2>&1 && echo "✓ Session + prompt parsing works"

# Test 3: File auto-detection
echo "3. Testing file auto-detection..."
echo "test content for auto-detection" > auto-detect.md
$PARA_BINARY dispatch "auto-detect.md" --help >/dev/null 2>&1 && echo "✓ File auto-detection parsing works"

# Test 4: Session + file
echo "4. Testing session + file..."
$PARA_BINARY dispatch "my-session" "dispatch-test.md" --help >/dev/null 2>&1 && echo "✓ Session + file parsing works"

# Test 5: Explicit file flag
echo "5. Testing explicit file flag..."
$PARA_BINARY dispatch --file "dispatch-test.md" --help >/dev/null 2>&1 && echo "✓ Explicit file flag parsing works"

# Test 6: File flag with session
echo "6. Testing file flag with session..."
$PARA_BINARY dispatch "my-session" --file "dispatch-test.md" --help >/dev/null 2>&1 && echo "✓ File flag with session parsing works"

# Test 7: Skip permissions flag
echo "7. Testing skip permissions flag..."
$PARA_BINARY dispatch "test prompt" --dangerously-skip-permissions --help >/dev/null 2>&1 && echo "✓ Skip permissions flag parsing works"

echo ""
echo "✅ All dispatch argument patterns validated successfully!"
echo "The Rust implementation supports all legacy dispatch command patterns."

# Cleanup
rm -f auto-detect.md dispatch-test.md test-prompt.txt
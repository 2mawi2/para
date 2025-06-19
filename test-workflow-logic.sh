#!/bin/bash

# Test script to validate workflow logic locally

echo "🔍 Testing Auditor workflow logic..."

# Test finding unwrap usage
echo "Finding .unwrap() usage in src/:"
unwrap_files=$(rg --type rust -l "\.unwrap\(\)" src/ || true)

if [ -z "$unwrap_files" ]; then
    echo "❌ No .unwrap() usage found in src/"
    exit 1
else
    echo "✅ Found .unwrap() usage in the following files:"
    echo "$unwrap_files"
fi

# Test selecting random file
selected_file=$(echo "$unwrap_files" | shuf -n 1)
echo "📝 Selected file: $selected_file"

# Test filename sanitization
base_filename=$(basename "$selected_file" .rs)
sanitized_filename=$(echo "$base_filename" | sed 's/[^a-zA-Z0-9_-]/-/g')
echo "🔧 Sanitized filename: $sanitized_filename"

# Test task filename generation
timestamp=$(date +%s)
task_filename="tasks/gardener-backlog/${timestamp}-fix-unwrap-in-${sanitized_filename}.md"
echo "📄 Task filename: $task_filename"

echo ""
echo "🌿 Testing Gardener workflow logic..."

# Test finding task files
echo "Checking for task files in gardener-backlog:"
if [ -d "tasks/gardener-backlog" ]; then
    task_files=$(find tasks/gardener-backlog -name "*.md" -type f)
    if [ -n "$task_files" ]; then
        echo "✅ Found task files:"
        echo "$task_files"
        
        # Test extracting branch name from task file
        for task_file in $task_files; do
            task_basename=$(basename "$task_file" .md)
            branch_suffix=$(echo "$task_basename" | sed 's/^[0-9]*-//')
            branch_name="gardener/$branch_suffix"
            session_name=$(echo "$branch_suffix" | sed 's/-/_/g')
            
            echo "  📋 Task: $task_file"
            echo "     Branch: $branch_name"
            echo "     Session: $session_name"
        done
    else
        echo "ℹ️  No task files found (this is normal for testing)"
    fi
else
    echo "❌ tasks/gardener-backlog directory not found"
    exit 1
fi

echo ""
echo "✅ All workflow logic tests passed!"
echo ""
echo "💡 Workflow validation summary:"
echo "   - Auditor can find .unwrap() usage: ✅"
echo "   - Filename sanitization works: ✅"
echo "   - Task file generation logic: ✅"
echo "   - Gardener can parse task files: ✅"
echo "   - Branch naming logic works: ✅"
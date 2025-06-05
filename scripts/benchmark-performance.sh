#!/usr/bin/env bash

# Para Session Cancellation Performance Benchmark
# Run this script to test session cancellation performance in your environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PARA_SCRIPT="$SCRIPT_DIR/../para.sh"

# Check if we're in a git repository
if ! git rev-parse --git-dir >/dev/null 2>&1; then
    echo "❌ Not in a Git repository. Please run this from within a Git repository."
    exit 1
fi

# Check if para.sh exists
if [ ! -f "$PARA_SCRIPT" ]; then
    echo "❌ para.sh not found at $PARA_SCRIPT"
    exit 1
fi

# Set test mode environment variables to prevent IDEs from launching during benchmarks
export IDE_CMD="true"  # Test stub for main IDE command
export CURSOR_CMD="true"  # Test stub for Cursor
export IDE_WRAPPER_CMD="true"  # Test stub for wrapper IDEs

echo "🚀 Para Session Cancellation Performance Benchmark"
echo "================================================="
echo ""
echo "ℹ️  Running in test mode (IDEs will not open during benchmark)"
echo ""

# Function to measure time in milliseconds
measure_time() {
    if command -v perl >/dev/null 2>&1; then
        perl -MTime::HiRes=time -E 'say int(time*1000)'
    else
        date +%s%3N 2>/dev/null || date +%s000
    fi
}

# Function to create and cancel a session
benchmark_session() {
    local session_name="$1"
    local add_files="$2"
    
    # Create session
    if ! "$PARA_SCRIPT" start "$session_name" >/dev/null 2>&1; then
        echo "❌ Failed to create session $session_name" >&2
        return 1
    fi
    
    # Find the session directory
    session_dir=""
    if [ -d "subtrees" ]; then
        session_dir=$(find subtrees -name "*$session_name*" -type d | head -1)
    fi
    
    if [ -z "$session_dir" ] || [ ! -d "$session_dir" ]; then
        echo "❌ Could not find session directory for $session_name" >&2
        return 1
    fi
    
    # Add files if requested
    if [ "$add_files" = "true" ]; then
        cd "$session_dir"
        for i in $(seq 1 10); do
            echo "Benchmark file $i content" > "benchmark-file-$i.txt"
        done
        git add . >/dev/null 2>&1
        git commit -m "Benchmark files" >/dev/null 2>&1
        cd - >/dev/null
    fi
    
    # Measure cancellation time
    start_time=$(measure_time)
    if ! "$PARA_SCRIPT" cancel "$session_name" >/dev/null 2>&1; then
        echo "❌ Failed to cancel session $session_name" >&2
        return 1
    fi
    end_time=$(measure_time)
    
    duration=$((end_time - start_time))
    echo "$duration"
    return 0
}

# Benchmark 1: Simple session cancellation
echo "📊 Benchmark 1: Simple session cancellation"
echo "-------------------------------------------"

simple_times=()
for i in $(seq 1 5); do
    echo "  Creating and cancelling session $i..." >&2
    duration=$(benchmark_session "bench-simple-$i" "false")
    if [ "$?" -eq 0 ]; then
        simple_times+=("$duration")
        echo "  Cycle $i: ${duration}ms"
    else
        echo "  Cycle $i: FAILED"
    fi
done

# Calculate average for simple sessions
if [ ${#simple_times[@]} -gt 0 ]; then
    total=0
    for time in "${simple_times[@]}"; do
        total=$((total + time))
    done
    avg_simple=$((total / ${#simple_times[@]}))
    echo "  Average: ${avg_simple}ms"
else
    echo "  ❌ All simple session tests failed"
    avg_simple=0
fi

echo ""

# Benchmark 2: Session with files cancellation
echo "📊 Benchmark 2: Session cancellation with files"
echo "-----------------------------------------------"

files_times=()
for i in $(seq 1 3); do
    echo "  Creating and cancelling session with files $i..." >&2
    duration=$(benchmark_session "bench-files-$i" "true")
    if [ "$?" -eq 0 ]; then
        files_times+=("$duration")
        echo "  Cycle $i: ${duration}ms"
    else
        echo "  Cycle $i: FAILED"
    fi
done

# Calculate average for sessions with files
if [ ${#files_times[@]} -gt 0 ]; then
    total=0
    for time in "${files_times[@]}"; do
        total=$((total + time))
    done
    avg_files=$((total / ${#files_times[@]}))
    echo "  Average: ${avg_files}ms"
else
    echo "  ❌ All session with files tests failed"
    avg_files=0
fi

echo ""

# Results summary
echo "📈 Performance Summary"
echo "====================="
echo ""

if [ "$avg_simple" -gt 0 ]; then
    echo "Simple session cancellation: ${avg_simple}ms average"
    if [ "$avg_simple" -lt 100 ]; then
        echo "  ✅ Excellent performance (< 100ms)"
    elif [ "$avg_simple" -lt 500 ]; then
        echo "  ✅ Good performance (< 500ms)"
    elif [ "$avg_simple" -lt 1000 ]; then
        echo "  ⚠️  Acceptable performance (< 1s)"
    else
        echo "  ❌ Slow performance (> 1s) - consider investigating"
    fi
fi

if [ "$avg_files" -gt 0 ]; then
    echo "Session with files cancellation: ${avg_files}ms average"
    if [ "$avg_files" -lt 200 ]; then
        echo "  ✅ Excellent performance (< 200ms)"
    elif [ "$avg_files" -lt 1000 ]; then
        echo "  ✅ Good performance (< 1s)"
    elif [ "$avg_files" -lt 2000 ]; then
        echo "  ⚠️  Acceptable performance (< 2s)"
    else
        echo "  ❌ Slow performance (> 2s) - consider investigating"
    fi
fi

echo ""

# Performance recommendations
if [ "$avg_simple" -gt 500 ] || [ "$avg_files" -gt 1000 ]; then
    echo "💡 Performance Tips:"
    echo "  • Ensure you're using an SSD for better I/O performance"
    echo "  • Close other Git/IDE processes that might be accessing the repository"
    echo "  • Consider running 'git gc' to optimize your repository"
    echo "  • Check if antivirus software is scanning the repository folder"
fi

echo ""
echo "🎯 Benchmark completed!"
echo ""
echo "ℹ️  If you experience consistently slow performance, please report this"
echo "   along with your system information (OS, Git version, storage type)." 
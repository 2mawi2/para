#!/bin/bash
# Para shim for containers - implements the Signal File Protocol
#
# This script acts as a para command inside containers, creating signal files
# that the host can detect and act upon.

PARA_DIR="/workspace/.para"
FINISH_SIGNAL="$PARA_DIR/finish_signal.json"
CANCEL_SIGNAL="$PARA_DIR/cancel_signal.json"
STATUS_FILE="$PARA_DIR/status.json"

# Ensure .para directory exists
mkdir -p "$PARA_DIR"

# Function to create timestamp in ISO 8601 format
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

# Function to escape JSON strings
json_escape() {
    echo "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\n/\\n/g; s/\r/\\r/g'
}

case "$1" in
    finish)
        shift
        MESSAGE=""
        BRANCH=""
        
        # Parse arguments
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --branch|-b)
                    BRANCH="$2"
                    shift 2
                    ;;
                *)
                    # Treat remaining arguments as the commit message
                    MESSAGE="$*"
                    break
                    ;;
            esac
        done
        
        # Validate message
        if [ -z "$MESSAGE" ]; then
            echo "Error: Commit message is required" >&2
            echo "Usage: para finish \"commit message\" [--branch <name>]" >&2
            exit 1
        fi
        
        # Escape message and branch for JSON
        MESSAGE_ESCAPED=$(json_escape "$MESSAGE")
        BRANCH_ESCAPED=$(json_escape "$BRANCH")
        
        # Create finish signal file
        if [ -n "$BRANCH" ]; then
            echo "{\"commit_message\":\"$MESSAGE_ESCAPED\",\"branch\":\"$BRANCH_ESCAPED\"}" > "$FINISH_SIGNAL"
        else
            echo "{\"commit_message\":\"$MESSAGE_ESCAPED\"}" > "$FINISH_SIGNAL"
        fi
        
        echo "✅ Finish signal sent to host. Host will stage and commit changes."
        ;;
        
    cancel)
        shift
        FORCE=false
        
        # Parse arguments
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --force|-f)
                    FORCE=true
                    shift
                    ;;
                *)
                    shift
                    ;;
            esac
        done
        
        # Create cancel signal file
        echo "{\"force\":$FORCE}" > "$CANCEL_SIGNAL"
        
        echo "✅ Cancel signal sent to host."
        ;;
        
    status)
        shift
        TASK=""
        TESTS=""
        CONFIDENCE=""
        TODOS=""
        BLOCKED=false
        
        # First argument is the task description
        if [ -n "$1" ] && [[ ! "$1" =~ ^-- ]]; then
            TASK="$1"
            shift
        fi
        
        # Parse remaining arguments
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --tests)
                    TESTS="$2"
                    shift 2
                    ;;
                --confidence)
                    CONFIDENCE="$2"
                    shift 2
                    ;;
                --todos)
                    TODOS="$2"
                    shift 2
                    ;;
                --blocked)
                    BLOCKED=true
                    shift
                    ;;
                *)
                    shift
                    ;;
            esac
        done
        
        # Validate task
        if [ -z "$TASK" ]; then
            echo "Error: Task description is required" >&2
            echo "Usage: para status \"task description\" [--tests status] [--confidence level] [--todos X/Y] [--blocked]" >&2
            exit 1
        fi
        
        # Escape values for JSON
        TASK_ESCAPED=$(json_escape "$TASK")
        TESTS_ESCAPED=$(json_escape "$TESTS")
        CONFIDENCE_ESCAPED=$(json_escape "$CONFIDENCE")
        TODOS_ESCAPED=$(json_escape "$TODOS")
        TIMESTAMP=$(get_timestamp)
        
        # Build JSON object
        JSON="{\"task\":\"$TASK_ESCAPED\",\"timestamp\":\"$TIMESTAMP\",\"blocked\":$BLOCKED"
        
        # Add optional fields if provided
        [ -n "$TESTS" ] && JSON="$JSON,\"tests\":\"$TESTS_ESCAPED\""
        [ -n "$CONFIDENCE" ] && JSON="$JSON,\"confidence\":\"$CONFIDENCE_ESCAPED\""
        [ -n "$TODOS" ] && JSON="$JSON,\"todos\":\"$TODOS_ESCAPED\""
        
        JSON="$JSON}"
        
        # Write status file
        echo "$JSON" > "$STATUS_FILE"
        
        echo "✅ Status updated."
        ;;
        
    *)
        echo "Para shim for containers"
        echo ""
        echo "Usage:"
        echo "  para finish \"commit message\" [--branch <name>]"
        echo "  para cancel [--force]"
        echo "  para status \"task\" [--tests status] [--confidence level] [--todos X/Y] [--blocked]"
        echo ""
        echo "This is a limited para implementation for use inside containers."
        echo "Commands create signal files that the host system processes."
        ;;
esac
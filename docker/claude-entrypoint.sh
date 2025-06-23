#!/bin/bash
# Enhanced entrypoint script for Claude container with Para support

# Set up git configuration
setup_git_config() {
    # Copy host git config if available
    if [ -f /workspace/.git/config ]; then
        # Extract user config from the workspace git config
        local user_name=$(git -C /workspace config user.name 2>/dev/null || echo "")
        local user_email=$(git -C /workspace config user.email 2>/dev/null || echo "")
        
        # Set global config if we found user info
        if [ -n "$user_name" ]; then
            git config --global user.name "$user_name"
        fi
        if [ -n "$user_email" ]; then
            git config --global user.email "$user_email"
        fi
    fi
    
    # Set basic git identity if not present
    if ! git config --global user.email >/dev/null 2>&1; then
        git config --global user.email "agent@para.docker"
    fi
    
    if ! git config --global user.name >/dev/null 2>&1; then
        git config --global user.name "Para Agent"
    fi
}

# Verify Para is available
check_para() {
    if command -v para >/dev/null 2>&1; then
        echo "✓ Para is available in container"
        # Show current session if set
        if [ -n "$PARA_SESSION" ]; then
            echo "  Current session: $PARA_SESSION"
        fi
        # Verify it works
        para --version >/dev/null 2>&1 && echo "  Para version check passed"
    else
        echo "⚠ Para binary not found - will be mounted from host"
    fi
}

# Set up the container environment
setup_git_config
check_para

# Execute the command passed to the container
exec "$@"
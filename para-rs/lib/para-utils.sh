#!/bin/bash

# Minimal stub functions for legacy test compatibility

is_known_command() {
    local cmd="$1"
    case "$cmd" in
        start|dispatch|finish|integrate|cancel|clean|list|ls|resume|recover|continue|config|completion)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Export functions so they're available to tests
export -f is_known_command
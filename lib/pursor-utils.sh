#!/usr/bin/env sh
# Utility functions for pursor

# Display usage information
usage() {
  cat >&2 <<EOF
Usage:
  pursor [session-name]           # create new session & open Cursor
  pursor merge "message"          # merge current session with commit message
  pursor list                     # list all active sessions (alias: ls)
  pursor continue                 # continue merge after resolving conflicts
  pursor cancel                   # cancel current session (alias: abort)
  pursor clean                    # clean up all sessions
  pursor resume [session-name]    # resume/reconnect to existing session

Examples:
  pursor                          # start new parallel session
  pursor feature-auth             # start named session "feature-auth"
  pursor merge "Add new feature"  # merge with commit message
  pursor continue                 # resume after fixing conflicts
  pursor resume feature-auth      # reconnect to named session
  pursor cancel                   # discard current session
EOF
  exit 1
}

# Print error message and exit
die() {
  echo "pursor: $*" >&2
  exit 1
}

# Check if argument is a known command
is_known_command() {
  case "$1" in
    list|ls|clean|--help|-h|merge|continue|cancel|abort|resume)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

# Validate session name format
validate_session_name() {
  session_name="$1"
  case "$session_name" in
    *[!a-zA-Z0-9_-]*) 
      die "session name can only contain letters, numbers, dashes, and underscores" 
      ;;
  esac
}

# Generate timestamp for session IDs
generate_timestamp() {
  date +%Y%m%d-%H%M%S
} 
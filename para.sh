#!/usr/bin/env sh
# para.sh - Parallel IDE Workflow Helper
# Main entry point that orchestrates the modular components

set -eu

# Determine script directory for sourcing libraries
SCRIPT_DIR="$(dirname "$0")"
case "$SCRIPT_DIR" in
/*) ;;
*) SCRIPT_DIR="$PWD/$SCRIPT_DIR" ;;
esac

# Source library modules
LIB_DIR="$SCRIPT_DIR/lib"
. "$LIB_DIR/para-config.sh"
# Early intercept: handle 'config edit' directly, bypass loading or validating the config file
if [ "$#" -ge 2 ] && [ "$1" = "config" ] && [ "$2" = "edit" ]; then
  # Inline 'config edit' without loading full config
  cmd="${EDITOR:-vi}"
  if [ -f "$CONFIG_FILE" ]; then
    # Support commands with arguments by using eval
    eval "$cmd \"$CONFIG_FILE\""
  else
    echo "No config file found. Run 'para config' to create one."
  fi
  exit 0
fi
. "$LIB_DIR/para-config-wizard.sh"
. "$LIB_DIR/para-utils.sh"
. "$LIB_DIR/para-git.sh"
. "$LIB_DIR/para-session.sh"
. "$LIB_DIR/para-ide.sh"
. "$LIB_DIR/para-backup.sh"
. "$LIB_DIR/para-commands.sh"

# Show version information
show_version() {
  version=$(git tag -l "v*" 2>/dev/null | sort -V | tail -1)
  if [ -z "$version" ]; then
    version="dev"
  fi
  echo "para $version"
}

# Initialize environment
need_git_repo
load_config
init_paths

# Command dispatch logic
main() {
  # Check for first run before handling commands (but skip for config commands)
  if [ "$#" -gt 0 ] && [ "$1" != "config" ]; then
    check_first_run
  fi

  # Handle commands or show usage
  if [ "$#" -eq 0 ]; then
    # No arguments - show usage
    usage
    return 0
  else
    # Handle known commands
    handle_command "$@"
    return $?
  fi
}

# Handle known commands
handle_command() {
  case "$1" in
  --help | -h)
    usage
    ;;

  --version | -v)
    show_version
    ;;

  start)
    handle_start_command "$@"
    ;;

  dispatch)
    handle_dispatch_command "$@"
    ;;

  dispatch-multi)
    handle_dispatch_multi_command "$@"
    ;;

  finish)
    handle_finish_command "$@"
    ;;

  integrate)
    handle_integrate_command "$@"
    ;;

  cancel | abort)
    handle_cancel_command "$@"
    ;;

  clean)
    clean_all_sessions
    ;;

  list | ls)
    list_sessions
    ;;

  resume)
    handle_resume_command "$@"
    ;;

  continue)
    handle_continue_command "$@"
    ;;

  recover)
    handle_recover_command "$@"
    ;;

  config)
    handle_config_command "$@"
    ;;

  completion | _completion_sessions | _completion_groups | _completion_branches)
    handle_completion_command "$@"
    ;;

  *)
    usage
    ;;
  esac
}

# Execute main function if script is run directly
if [ "$0" != "${0#*/}" ] || [ "$0" = "./para.sh" ] || [ "$0" = "para.sh" ]; then
  main "$@"
fi

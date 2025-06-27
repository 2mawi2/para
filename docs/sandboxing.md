# Sandboxing Support in Para

Para now includes macOS Seatbelt sandboxing support for the Claude CLI, providing kernel-level security boundaries to restrict file system access when running AI agents.

## Overview

When sandboxing is enabled, Claude CLI runs within a restricted environment that:
- Allows reading files from anywhere on the system
- Restricts file writes to specific directories only
- Permits all network access
- Allows process execution and forking

## Configuration

Add the following to your Para configuration file:

```json
{
  "sandbox": {
    "enabled": true,
    "profile": "permissive-open"
  }
}
```

## Allowed Write Locations

With the `permissive-open` profile, Claude can only write to:
- The current Para worktree directory
- System temporary directories
- User's cache directory
- Para configuration directories (`~/.para`)
- Claude configuration directories (`~/.claude`, `~/.config/claude`)
- Standard output streams (stdout, stderr, tty)

## Requirements

- macOS operating system
- `sandbox-exec` command available (included with macOS)

## How It Works

1. When you start a Para session with sandboxing enabled, Para checks if you're on macOS and if `sandbox-exec` is available
2. If both conditions are met, Para wraps the Claude command with `sandbox-exec`
3. The sandbox profile is extracted to a temporary location
4. Dynamic parameters (worktree path, temp dir, etc.) are passed to the sandbox
5. Claude runs within the sandboxed environment

## Security Benefits

- **Prevents accidental file modifications** outside the project directory
- **Protects system files** from unintended changes
- **Isolates project work** to the specific worktree
- **Maintains full read access** for code analysis and understanding

## Limitations

- Only available on macOS (Linux sandboxing may be added in the future)
- Network access is not restricted in the permissive-open profile
- Process execution is allowed (Claude can still run commands)

## Troubleshooting

If sandboxing fails to apply:
1. Check that you're running on macOS
2. Verify `sandbox-exec` is available: `which sandbox-exec`
3. Check Para logs for sandbox-related warnings
4. The system will fall back to running without sandboxing if setup fails

## Future Enhancements

- Additional sandbox profiles (restrictive-closed, network-isolated)
- Linux sandboxing support via bubblewrap
- Custom user-provided sandbox profiles
- Resource usage limits (CPU, memory, disk)
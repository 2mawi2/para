# Implementation Notes: SignalFileWatcher in Start Command

## Changes Made

Added SignalFileWatcher spawning to the `start` command for container sessions, matching the behavior in the `dispatch` command.

### Modified Files
- `src/cli/commands/start.rs`: Added SignalFileWatcher spawn for container sessions (lines 52-57)

### Implementation Details

When starting a container session with `para start --container`, the command now:
1. Creates the Docker container session
2. Launches the IDE connected to the container
3. **Spawns a SignalFileWatcher** to monitor for finish/cancel signals from the container

The watcher handle is stored as `_watcher_handle` to keep it alive for the duration of the session.

## Testing

- Start command tests pass: `just test start` ✅
- Watcher tests pass: `just test watcher` ✅
- Dispatch tests pass: `just test dispatch` ✅
- Code compiles successfully: `cargo build` ✅

## Known Issues

There is an unrelated test failure in `core::git::diff::tests` that appears to be environment-specific or flaky. This is not related to the SignalFileWatcher changes.

## Verification

The implementation is consistent with how `dispatch.rs` handles container sessions (see lines 88-94 in dispatch.rs).
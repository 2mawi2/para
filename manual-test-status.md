# Manual Test Guide for Container Status Updates

## Prerequisites
1. Build the latest para binary: `just build`
2. Make sure no daemon is running: `./target/debug/para daemon stop`

## Test Steps

### 1. Start the daemon
```bash
./target/debug/para daemon start
```

### 2. Create a test container session

In a test repository, create a container session:
```bash
# In your test repo
para start test-status --container
```

### 3. Inside the container, run status updates

Once inside the container:
```bash
# Update status with all fields
para status "Implementing auth" --tests failed --confidence high --todos 3/5

# Check the status file was created on the host
# (from another terminal on the host)
ls -la .para/state/*.status.json
cat .para/state/test-status.status.json

# Update status again
para status "Fixed tests" --tests passed --confidence high --todos 5/5

# Check it was updated
cat .para/state/test-status.status.json
```

### 4. Verify the status file contains:
- session_name
- current_task (from the status message)
- test_status (passed/failed/unknown)
- confidence (high/medium/low)
- todos_completed and todos_total
- last_update timestamp

### 5. Test edge cases
```bash
# Status without todos
para status "Reviewing code" --tests passed --confidence medium

# Status with blocked flag
para status "Blocked on API" --tests unknown --confidence low --blocked
```

### 6. Clean up
```bash
# Exit container and cancel session
para cancel

# Stop daemon
para daemon stop
```

## Expected Results
- Status files should be created in `.para/state/[session-name].status.json`
- Each update should overwrite the previous status
- The daemon should process updates within 1-2 seconds
- Status files should persist after container exits

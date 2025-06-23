# Docker Container Status Communication Protocol

## Overview

This document defines the protocol for communication between Docker containers and the para host system. The protocol uses shared JSON files in the `.para/state/` directory to enable bidirectional status updates and command passing.

## Directory Structure

```
.para/
└── state/
    ├── {session-name}.state         # Para session state (existing)
    ├── {session-name}.docker        # Docker container metadata
    └── docker/
        ├── {session-name}/
        │   ├── status.json          # Container → Host status
        │   ├── commands.json        # Host → Container commands
        │   ├── logs/                # Structured logs
        │   └── metrics.json         # Resource usage metrics
        └── shared/
            └── registry.json        # Active container registry
```

## Status Communication Files

### 1. Container Status (`status.json`)

Updated by containers to report their current state:

```json
{
  "timestamp": "2024-01-20T10:30:00Z",
  "status": "running",
  "health": "healthy",
  "phase": "building",
  "progress": {
    "current": 45,
    "total": 100,
    "message": "Compiling dependencies..."
  },
  "services": {
    "database": "running",
    "redis": "starting",
    "api": "stopped"
  },
  "errors": [],
  "warnings": [
    {
      "timestamp": "2024-01-20T10:29:00Z",
      "message": "Low disk space: 90% used"
    }
  ],
  "custom": {
    "test_results": {
      "passed": 142,
      "failed": 3,
      "skipped": 7
    }
  }
}
```

### 2. Command Queue (`commands.json`)

Written by host, consumed by containers:

```json
{
  "commands": [
    {
      "id": "cmd-123",
      "timestamp": "2024-01-20T10:31:00Z",
      "type": "exec",
      "command": "npm test",
      "args": ["--coverage"],
      "env": {
        "NODE_ENV": "test"
      },
      "working_dir": "/workspace",
      "timeout_seconds": 300,
      "priority": "normal"
    },
    {
      "id": "cmd-124",
      "timestamp": "2024-01-20T10:32:00Z",
      "type": "signal",
      "signal": "reload_config"
    }
  ],
  "processed": ["cmd-120", "cmd-121", "cmd-122"]
}
```

### 3. Container Registry (`shared/registry.json`)

Maintained by DockerManager:

```json
{
  "containers": [
    {
      "session_name": "feature-auth",
      "container_id": "abc123def456",
      "status": "running",
      "started_at": "2024-01-20T10:00:00Z",
      "host_port_mappings": {
        "3000": "3000",
        "5432": "15432"
      },
      "health_endpoint": "http://localhost:3000/health"
    }
  ],
  "last_updated": "2024-01-20T10:33:00Z"
}
```

## Container-Side Implementation

Containers should include a lightweight status agent that:

1. **Monitors** the command queue file for new commands
2. **Executes** commands and updates status
3. **Reports** progress and health status
4. **Handles** signals for graceful operations

Example agent script (`/para/status-agent.sh`):

```bash
#!/bin/bash
# Para container status agent

STATUS_DIR="/workspace/.para/state/docker/${PARA_SESSION_NAME}"
STATUS_FILE="${STATUS_DIR}/status.json"
COMMAND_FILE="${STATUS_DIR}/commands.json"

# Ensure directories exist
mkdir -p "${STATUS_DIR}/logs"

# Update status function
update_status() {
    local phase="$1"
    local message="$2"
    local progress_current="${3:-0}"
    local progress_total="${4:-100}"
    
    cat > "$STATUS_FILE" <<EOF
{
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "running",
  "health": "healthy",
  "phase": "$phase",
  "progress": {
    "current": $progress_current,
    "total": $progress_total,
    "message": "$message"
  }
}
EOF
}

# Main loop
while true; do
    # Check for new commands
    if [ -f "$COMMAND_FILE" ]; then
        # Process commands (simplified)
        jq -r '.commands[] | select(.id as $id | .processed | index($id) | not)' "$COMMAND_FILE" | while read -r cmd; do
            # Execute command and update status
            echo "Processing command: $cmd"
        done
    fi
    
    # Update heartbeat
    update_status "idle" "Waiting for commands..." 0 0
    
    sleep 5
done
```

## Integration Points

### 1. Session Start

When a Docker container is created for a session:

1. DockerManager creates status directory structure
2. Mounts `.para/state/docker/{session}/` as a volume
3. Injects `PARA_SESSION_NAME` environment variable
4. Starts container with status agent

### 2. Status Monitoring

The para monitor command can:

1. Read container status files
2. Display container health and progress
3. Show service statuses within containers
4. Alert on errors or warnings

### 3. Command Execution

When using `para docker exec`:

1. Command is added to the queue
2. Container agent picks up and executes
3. Status is updated with results
4. Output is captured in logs

### 4. Session Finish

When finishing a session with Docker:

1. Send shutdown signal via command queue
2. Wait for graceful shutdown
3. Collect final status and logs
4. Archive or clean up status files

## Volume Mappings

Standard volume mappings for status communication:

```yaml
volumes:
  # Para state directory (read-write for status updates)
  - ${WORKTREE}/.para/state/docker/${SESSION_NAME}:/para/status:rw
  
  # Shared registry (read-only)
  - ${WORKTREE}/.para/state/docker/shared:/para/shared:ro
  
  # Project workspace
  - ${WORKTREE}:/workspace:rw
```

## Security Considerations

1. **File Permissions**: Status directories should be writable only by para and container user
2. **Command Validation**: Containers should validate commands before execution
3. **Resource Limits**: Status files should have size limits to prevent DoS
4. **Sensitive Data**: Never include secrets or credentials in status files

## Error Handling

1. **Missing Files**: Treat as empty/default state
2. **Invalid JSON**: Log error and skip, don't crash
3. **Write Failures**: Retry with exponential backoff
4. **Lock Files**: Use atomic writes to prevent corruption

## Future Extensions

1. **Bidirectional Streaming**: WebSocket or gRPC for real-time communication
2. **Binary Protocol**: For high-performance scenarios
3. **Encryption**: For sensitive status information
4. **Multi-Container Orchestration**: Coordinate between multiple containers
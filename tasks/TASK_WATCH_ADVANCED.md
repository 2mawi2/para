# Task: Para Watch - Advanced Integration Implementation

Create a comprehensive monitoring system for Para that integrates with real session data and system processes.

## Comprehensive Feature Set

Build a full monitoring solution that provides both real-time watching and detailed session analysis.

## Core Components

### 1. Session Data Integration
- Read actual session data from `.para/sessions/` directory
- Parse session metadata and state from JSON files
- Track session lifecycle and state transitions
- Monitor git worktree status and changes

### 2. IDE Process Detection
- Detect running IDE processes (VS Code, Cursor, etc.)
- Match IDE instances to Para sessions by working directory
- Monitor IDE process lifecycle (start/stop events)
- Cross-platform process detection (macOS, Linux, Windows)

### 3. Real-time Monitoring
- File system watching for session state changes
- Process monitoring for IDE lifecycle events
- Git repository monitoring for commits and changes
- Automatic refresh when changes detected

### 4. Multiple Display Modes

#### Interactive TUI Mode (`para watch`)
```rust
// Rich terminal interface with:
- Real-time session updates
- Keyboard shortcuts for IDE launching
- Session detail views
- Git status integration
- Process monitoring
```

#### Status Command (`para status`)
```rust
// Quick status overview:
- Current session count by state
- Active IDE processes
- Recent activity summary
- System health check
```

#### JSON Output (`para status --json`)
```rust
// Machine-readable format for integration:
{
  "sessions": [...],
  "statistics": {...},
  "system_info": {...}
}
```

## Technical Architecture

### Data Layer
```rust
// src/core/monitoring/
mod session_tracker;    // Session state management
mod process_monitor;    // IDE process detection
mod git_watcher;        // Git repository monitoring
mod system_info;        // System health and metrics

pub struct MonitoringSystem {
    session_tracker: SessionTracker,
    process_monitor: ProcessMonitor,
    git_watcher: GitWatcher,
    system_info: SystemInfo,
}
```

### Session State Management
```rust
struct SessionDetails {
    // Core session info
    name: String,
    agent_name: Option<String>,
    task_description: String,
    
    // State tracking
    current_state: SessionState,
    state_history: Vec<StateTransition>,
    
    // Process info
    ide_process: Option<ProcessInfo>,
    working_directory: PathBuf,
    
    // Git info
    branch_name: String,
    commit_count: u32,
    last_commit: Option<DateTime<Utc>>,
    
    // Timing
    created_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    state_durations: HashMap<SessionState, Duration>,
}

enum SessionState {
    Working,
    AIReview { attempt: u8, started_at: DateTime<Utc> },
    HumanReview { assigned_at: DateTime<Utc> },
    Completed { merged_at: DateTime<Utc> },
    Failed { error: String },
}
```

### Process Detection
```rust
struct ProcessInfo {
    pid: u32,
    name: String,
    command_line: String,
    working_directory: PathBuf,
    cpu_usage: f32,
    memory_usage: u64,
    started_at: DateTime<Utc>,
}

impl ProcessMonitor {
    fn detect_ide_processes(&self) -> Vec<ProcessInfo>;
    fn match_process_to_session(&self, process: &ProcessInfo) -> Option<String>;
    fn launch_ide_for_session(&self, session_name: &str) -> Result<()>;
}
```

### Real-time Updates
```rust
use notify::{Watcher, RecursiveMode, DebouncedEvent};
use tokio::sync::mpsc;

struct EventSystem {
    session_events: mpsc::Receiver<SessionEvent>,
    process_events: mpsc::Receiver<ProcessEvent>,
    git_events: mpsc::Receiver<GitEvent>,
}

enum SessionEvent {
    StateChanged { session: String, new_state: SessionState },
    SessionCreated { session: String },
    SessionCompleted { session: String },
}
```

## Implementation Requirements

### 1. Cross-platform Support
- macOS: Use `sysctl` and `ps` for process detection
- Linux: Use `/proc` filesystem for process info
- Windows: Use Windows API for process enumeration

### 2. Performance Optimization
- Efficient file system watching
- Minimal CPU usage during monitoring
- Smart caching of session data
- Debounced event handling

### 3. Error Handling
- Graceful degradation when IDE detection fails
- Fallback modes for unsupported platforms
- Clear error messages for common issues
- Recovery from corrupted session data

### 4. Configuration Integration
- Respect user's IDE preferences from config
- Configurable refresh intervals
- Optional features (process monitoring, git watching)
- Debug mode for troubleshooting

## Deliverables

1. **Core monitoring system** (`src/core/monitoring/`)
2. **CLI commands** (`src/cli/commands/status.rs`, `src/cli/commands/watch.rs`)
3. **TUI interface** using Ratatui for interactive mode
4. **Integration tests** with mock data and real session scenarios
5. **Documentation** for configuration and usage

## Success Criteria

- Accurate detection of IDE processes matched to sessions
- Real-time updates when session state changes
- Clean fallback when advanced features aren't available
- Performance: <5% CPU usage during monitoring
- Cross-platform compatibility (at least macOS + Linux)
- Integration with existing Para workflow

## Testing Strategy

1. **Unit tests** for each monitoring component
2. **Integration tests** with real session data
3. **Cross-platform testing** on different systems
4. **Performance testing** with many active sessions
5. **User acceptance testing** with real development workflows

Focus on building a robust, performant system that enhances the Para development experience without getting in the way.

When complete, run: para integrate "Add comprehensive para monitoring system"
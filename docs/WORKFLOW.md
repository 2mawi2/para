# Para Workflow Documentation

This document explains the Para session management workflow and state transitions using visual diagrams.

> **Note**: For detailed information about IDE window management during integration and conflict resolution, see [IDE_BEHAVIOR.md](./IDE_BEHAVIOR.md).

## Session State Diagram

```mermaid
stateDiagram-v2
    [*] --> Idle: para ready
    
    Idle --> Starting: para start <session-name>
    Idle --> Dispatching: para dispatch <prompt>
    
    Starting --> Active: worktree + branch created
    Dispatching --> Active: session auto-created
    Active --> Working: IDE opened
    
    Working --> Finishing: para finish <message>
    Working --> Canceling: para cancel
    Working --> Pausing: exit IDE (session preserved)
    
    Finishing --> Committed: changes staged & committed
    Canceling --> Cancelled: session deleted
    Pausing --> Paused: session preserved
    
    Committed --> Idle: back to main branch
    Cancelled --> Idle: session removed
    Paused --> Recovering: para recover <session-name>
    
    Recovering --> Active: session restored
```

## Command Flow Diagram

```mermaid
flowchart TD
    A[User Input] --> B{Command Type}
    
    B --> C[para start]
    B --> D[para dispatch]
    B --> F[para finish]
    B --> G[para cancel]
    B --> H[para recover]
    
    C --> C1[Create worktree]
    C1 --> C2[Create branch]
    C2 --> C3[Open IDE]
    C3 --> I[Active Session]
    
    D --> D1[Auto-generate session name]
    D1 --> D2[Create worktree & branch]
    D2 --> D3[Open IDE with prompt]
    D3 --> I
    
    I --> F
    
    F --> F1[Auto-stage all changes]
    F1 --> F2[Create commit]
    F2 --> F3[Switch to main]
    F3 --> F4[Clean up worktree]
    F4 --> K[Session Complete]
    
    I --> G
    
    G --> G1[Confirm deletion]
    G1 --> G2[Remove worktree]
    G2 --> G3[Delete branch]
    G3 --> L[Session Cancelled]
    
    H --> H1[Check session exists]
    H1 --> H2[Restore worktree]
    H2 --> H3[Switch to branch]
    H3 --> H4[Open IDE]
    H4 --> I
```

## Session Lifecycle

```mermaid
sequenceDiagram
    participant User
    participant Para
    participant Git
    participant IDE
    
    User->>Para: para start feature-auth
    Para->>Git: Create worktree subtrees/para/feature-auth
    Para->>Git: Create branch para/20250608-175855
    Para->>Git: Switch to new branch
    Para->>IDE: Launch IDE in session directory
    
    Note over User,IDE: Development work happens
    
    User->>Para: para finish "Implement OAuth" --branch feature/oauth
    Para->>Git: Stage all changes (git add .)
    Para->>Git: Commit with message
    Para->>Git: Create/switch to custom branch name
    Para->>Git: Switch back to main
    Para->>Git: Remove worktree
    Para->>User: Session complete
```


## File Input Workflow

```mermaid
flowchart TD
    A[User has complex requirements] --> B[Create prompt file]
    B --> C[para dispatch --file requirements.txt]
    C --> D[Para reads file content]
    D --> E[Create session with file as prompt]
    E --> F[IDE opens with full context]
    F --> G[AI processes requirements]
    G --> H[Implementation begins]
    H --> I[para finish 'Complete requirements' --branch feature/complex-task]
```

## Error Handling States

```mermaid
stateDiagram-v2
    [*] --> Command
    Command --> Validating: check prerequisites
    
    Validating --> Error_NoGit: not in git repo
    Validating --> Error_DirtyTree: uncommitted changes
    Validating --> Error_SessionExists: session name conflict
    Validating --> Success: all checks pass
    
    Error_NoGit --> [*]: exit with error
    Error_DirtyTree --> [*]: exit with error  
    Error_SessionExists --> [*]: exit with error
    
    Success --> Executing: proceed with command
    Executing --> Complete: command successful
    Executing --> Error_Runtime: runtime failure
    
    Error_Runtime --> Cleanup: attempt recovery
    Cleanup --> [*]: exit with error
    
    Complete --> [*]: success
```

## Configuration States

```mermaid
stateDiagram-v2
    [*] --> Unconfigured: first run
    
    Unconfigured --> Configuring: para config
    Configuring --> ConfigWizard: interactive setup
    ConfigWizard --> Configured: save preferences
    
    Configured --> Reconfiguring: para config
    Configured --> AutoDetecting: para config auto
    Configured --> Viewing: para config show
    Configured --> Editing: para config edit
    
    Reconfiguring --> ConfigWizard
    AutoDetecting --> Configured: IDE detected
    Viewing --> Configured: display settings
    Editing --> Configured: manual changes
    
    Configured --> Operating: normal usage
    Operating --> Configured: config changes needed
```
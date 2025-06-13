# IDE Behavior and Integration Workflow

This document explains how Para manages IDE windows during different operations, particularly focusing on the integration workflow and conflict resolution.

## Overview

Para intelligently manages IDE windows based on the operation outcome:
- **Success**: IDE closes automatically when work is complete
- **Conflicts/Failures**: IDE remains open for user intervention

## Command Behaviors

### `para finish`

The finish command closes the IDE optimistically before attempting git operations:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Execute   │     │  Close IDE  │     │   Perform   │
│   Finish    │────▶│   Window    │────▶│     Git     │
│   Command   │     │             │     │ Operations  │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                                               ▼
                                         ┌─────────────┐
                                         │   Success   │────▶ Done
                                         │      or     │
                                         │   Failure   │
                                         └─────────────┘
```

### `para integrate`

The integrate command keeps IDE open when conflicts are detected:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Execute   │     │   Attempt   │     │   Check     │
│  Integrate  │────▶│ Integration │────▶│   Result    │
│   Command   │     │             │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          ▼                    ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │   Success   │     │  Conflicts  │     │   Failed    │
                    │             │     │  Detected   │     │             │
                    └─────────────┘     └─────────────┘     └─────────────┘
                          │                    │                    │
                          ▼                    ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │  Close IDE  │     │  Keep IDE   │     │  Keep IDE   │
                    │   & Clean   │     │    Open     │     │    Open     │
                    └─────────────┘     └─────────────┘     └─────────────┘
```

### `para continue`

The continue command is used after resolving conflicts:

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Execute   │     │    Check    │     │  Conflicts  │
│  Continue   │────▶│  Conflicts  │────▶│  Resolved?  │
│   Command   │     │   Status    │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          ▼                    ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │     Yes     │     │     No      │     │    New      │
                    │  Continue   │     │   Return    │     │  Conflicts  │
                    │Integration │     │   Error     │     │  Detected   │
                    └─────────────┘     └─────────────┘     └─────────────┘
                          │                    │                    │
                          ▼                    ▼                    ▼
                    ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
                    │  Success?   │     │  IDE Stays  │     │  IDE Stays  │
                    │             │     │    Open     │     │    Open     │
                    └─────────────┘     └─────────────┘     └─────────────┘
                          │
                    ┌─────┴─────┐
                    ▼           ▼
              ┌─────────┐ ┌─────────┐
              │   Yes   │ │   No    │
              │Close IDE│ │IDE Stays│
              └─────────┘ └─────────┘
```

## Integration Workflow

The complete integration workflow with conflict resolution:

```
┌───────────────────────────────────────────────────────────────────────────┐
│                          Integration Workflow                              │
└───────────────────────────────────────────────────────────────────────────┘

1. Start Integration
   ┌─────────────┐
   │    para     │
   │  integrate  │
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │   Prepare   │────▶│   Execute   │────▶│   Result?   │
   │   Branches  │     │  Strategy   │     │             │
   └─────────────┘     └─────────────┘     └─────────────┘
                                                  │
                    ┌─────────────────────────────┼─────────────────────────┐
                    ▼                             ▼                         ▼
             ┌─────────────┐               ┌─────────────┐          ┌─────────────┐
             │   Success   │               │  Conflicts  │          │   Failed    │
             │             │               │             │          │             │
             └──────┬──────┘               └──────┬──────┘          └──────┬──────┘
                    │                             │                         │
                    ▼                             ▼                         ▼
             ┌─────────────┐               ┌─────────────┐          ┌─────────────┐
             │  Close IDE  │               │  Open/Keep  │          │   Return    │
             │  & Cleanup  │               │  IDE Open   │          │    Error    │
             └─────────────┘               └──────┬──────┘          └─────────────┘
                                                  │
2. Resolve Conflicts                              ▼
                                           ┌─────────────┐
                                           │    User     │
                                           │  Resolves   │
                                           │  Conflicts  │
                                           └──────┬──────┘
                                                  │
3. Continue Integration                           ▼
                                           ┌─────────────┐
                                           │    para     │
                                           │  continue   │
                                           └──────┬──────┘
                                                  │
                                                  ▼
                                           ┌─────────────┐
                                           │   Check     │
                                           │  Conflicts  │
                                           └──────┬──────┘
                                                  │
                    ┌─────────────────────────────┼─────────────────────────┐
                    ▼                             ▼                         ▼
             ┌─────────────┐               ┌─────────────┐          ┌─────────────┐
             │  Resolved   │               │   Still     │          │    New      │
             │  Continue   │               │  Conflicts  │          │  Conflicts  │
             └──────┬──────┘               └──────┬──────┘          └──────┬──────┘
                    │                             │                         │
                    ▼                             ▼                         ▼
             ┌─────────────┐               ┌─────────────┐          ┌─────────────┐
             │Integration  │               │   Return    │          │   Return    │
             │ Complete?   │               │   Error     │          │   Error     │
             └──────┬──────┘               │ (IDE Open)  │          │ (IDE Open)  │
                    │                      └─────────────┘          └─────────────┘
              ┌─────┴─────┐
              ▼           ▼
        ┌─────────┐ ┌─────────┐
        │   Yes   │ │   No    │
        │Close IDE│ │IDE Open │
        └─────────┘ └─────────┘
```

## Key Principles

1. **User-Friendly**: IDE stays open when user action is needed
2. **Clean Completion**: IDE closes when work is successfully done
3. **Wrapper Mode**: When running inside VS Code/Cursor, IDE is never closed
4. **Consistent Behavior**: Similar patterns across all commands

## Examples

### Successful Integration
```bash
$ para integrate feature-branch
🔄 Integrating session 'feature-branch' into 'main'
📋 Using squash strategy
✅ Integration completed successfully!
🧹 Cleaning up session...
# IDE closes automatically
```

### Integration with Conflicts
```bash
$ para integrate feature-branch
🔄 Integrating session 'feature-branch' into 'main'
📋 Using rebase strategy
⚠️  Integration paused due to conflicts
📁 Conflicted files:
   • src/main.rs
   • src/lib.rs
🚀 Opening IDE for conflict resolution...
# IDE stays open or opens for conflict resolution

$ # User resolves conflicts in IDE
$ para continue
🔄 All conflicts resolved. Continuing integration...
✅ Integration completed successfully!
🧹 Cleaning up session...
# IDE closes after successful completion
```

### Failed Continue
```bash
$ para continue
⚠️  Cannot continue: 2 conflicts remain unresolved
📁 Conflicted files:
   • src/main.rs
   • src/lib.rs
# IDE stays open for continued conflict resolution
```

## Configuration

The IDE closing behavior respects the wrapper mode configuration:

```json
{
  "ide": {
    "wrapper": {
      "enabled": true,
      "name": "cursor"
    }
  }
}
```

When `wrapper.enabled` is `true`, Para never attempts to close the IDE window, as it's running inside the parent IDE.
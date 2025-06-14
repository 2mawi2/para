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



## Workflow with Conflict Resolution

The complete finish workflow showing conflict resolution handling:

```
┌───────────────────────────────────────────────────────────────────────────┐
│                          Para Finish Workflow                              │
└───────────────────────────────────────────────────────────────────────────┘

1. Execute Finish Command
   ┌─────────────┐
   │    para     │
   │   finish    │
   └──────┬──────┘
          │
          ▼
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
   │   Stage     │────▶│   Create    │────▶│   Result?   │
   │   Changes   │     │   Branch    │     │             │
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
             │  Close IDE  │               │  Keep IDE   │          │   Return    │
             │  & Cleanup  │               │    Open     │          │    Error    │
             └─────────────┘               └──────┬──────┘          └─────────────┘
                                                  │
2. Resolve Conflicts                              ▼
                                           ┌─────────────┐
                                           │    User     │
                                           │  Resolves   │
                                           │  Conflicts  │
                                           └──────┬──────┘
                                                  │
3. Manual Conflict Resolution                     ▼
                                           ┌─────────────┐
                                           │    User     │
                                           │   Manually  │
                                           │  Resolves   │
                                           │  Conflicts  │
                                           │     and     │
                                           │  Completes  │
                                           │    Work     │
                                           └──────┬──────┘
                                                  │
                                                  ▼
                                           ┌─────────────┐
                                           │   Manual    │
                                           │  Completion │
                                           │ (IDE Stays  │
                                           │    Open)    │
                                           └─────────────┘
```

## Key Principles

1. **User-Friendly**: IDE stays open when user action is needed
2. **Clean Completion**: IDE closes when work is successfully done
3. **Wrapper Mode**: When running inside VS Code/Cursor, IDE is never closed
4. **Consistent Behavior**: Similar patterns across all commands

## Examples

### Successful Finish
```bash
$ para finish "Add feature implementation"
🔄 Creating branch for session 'feature-branch'
📋 Staging all changes
✅ Branch created successfully!
🧹 Cleaning up session...
# IDE closes automatically
```

### Finish with Conflicts
```bash
$ para finish "Add conflicting changes"
🔄 Creating branch for session 'feature-branch'
📋 Staging all changes
⚠️  Branch creation paused due to conflicts
📁 Conflicted files:
   • src/main.rs
   • src/lib.rs
🚀 IDE stays open for conflict resolution...
# User resolves conflicts and completes work manually
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
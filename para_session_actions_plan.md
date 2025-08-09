# Para Session Actions Implementation Plan

## Overview
This plan outlines the implementation of Cancel and Finish button functionality for para sessions in the UI, enabling users to complete or cancel sessions directly from the interface.

## Requirements Analysis

### Para CLI Commands
1. **para finish**: Complete session and create feature branch
   - Requires: commit message
   - Optional: session ID (auto-detects if not provided), custom branch name
   - Creates a commit and feature branch from session work

2. **para cancel**: Cancel session (moves to archive)
   - Optional: session ID (auto-detects if not provided)
   - Optional: --force flag for uncommitted changes
   - Moves session to archive, cleans up worktree

## Current UI Architecture

### Existing Components
- **Sidebar.tsx**: Contains session list with placeholder Cancel/Finish buttons (lines 175-177)
- **App.tsx**: Main app component that could host global action handlers
- **useSessionWorkspace hook**: Manages terminal state for sessions
- **Para CLI integration**: Already exists in `src-tauri/src/para_cli/`

### Current Event System
- Custom events dispatched via `window.dispatchEvent`
- Selection events: `para-ui:selection` 
- Terminal events: `terminal-output-{id}`
- Para update events: `para-sessions-updated`

## Proposed Architecture

### 1. Event-Driven Approach (Recommended)
Create a global session action handler in App.tsx that listens for session action events dispatched from anywhere in the UI.

```typescript
// Event types
interface SessionActionEvent {
  action: 'finish' | 'cancel'
  sessionId: string
  sessionName: string
  worktreePath: string
}

// Dispatch from Sidebar buttons
window.dispatchEvent(new CustomEvent('para-ui:session-action', { 
  detail: { action: 'finish', sessionId, sessionName, worktreePath }
}))
```

### 2. Backend Integration

#### New Tauri Commands
```rust
// src-tauri/src/para_cli/client.rs
pub async fn finish_session(&self, session_id: &str, message: &str, branch: Option<&str>) -> Result<()>
pub async fn cancel_session(&self, session_id: &str, force: bool) -> Result<()>

// src-tauri/src/main.rs
#[tauri::command]
async fn para_finish_session(session_id: String, message: String, branch: Option<String>) -> Result<(), String>

#[tauri::command]
async fn para_cancel_session(session_id: String, force: bool) -> Result<(), String>
```

### 3. UI Components

#### Session Action Handler (App.tsx)
```typescript
function SessionActionHandler() {
  const handleSessionAction = async (event: CustomEvent<SessionActionEvent>) => {
    const { action, sessionId, sessionName } = event.detail
    
    if (action === 'finish') {
      // Show modal for commit message
      const message = await showFinishModal(sessionName)
      if (message) {
        await invoke('para_finish_session', { sessionId, message })
        await invoke('refresh_para_sessions')
      }
    } else if (action === 'cancel') {
      // Show confirmation dialog
      const confirmed = await showCancelConfirmation(sessionName)
      if (confirmed) {
        await invoke('para_cancel_session', { sessionId, force: false })
        await invoke('refresh_para_sessions')
      }
    }
  }
  
  useEffect(() => {
    window.addEventListener('para-ui:session-action', handleSessionAction)
    return () => window.removeEventListener('para-ui:session-action', handleSessionAction)
  }, [])
}
```

#### Finish Session Modal
```typescript
interface FinishSessionModalProps {
  open: boolean
  sessionName: string
  onConfirm: (message: string, branch?: string) => void
  onCancel: () => void
}

function FinishSessionModal({ open, sessionName, onConfirm, onCancel }: FinishSessionModalProps) {
  const [message, setMessage] = useState('')
  const [customBranch, setCustomBranch] = useState('')
  
  return (
    <Modal open={open}>
      <h2>Finish Session: {sessionName}</h2>
      <textarea 
        placeholder="Commit message (required)"
        value={message}
        onChange={(e) => setMessage(e.target.value)}
      />
      <input 
        placeholder="Custom branch name (optional)"
        value={customBranch}
        onChange={(e) => setCustomBranch(e.target.value)}
      />
      <button onClick={() => onConfirm(message, customBranch)}>Finish</button>
      <button onClick={onCancel}>Cancel</button>
    </Modal>
  )
}
```

#### Cancel Session Confirmation
```typescript
interface CancelConfirmationProps {
  open: boolean
  sessionName: string
  hasUncommittedChanges: boolean
  onConfirm: (force: boolean) => void
  onCancel: () => void
}

function CancelConfirmation({ open, sessionName, hasUncommittedChanges, onConfirm, onCancel }: CancelConfirmationProps) {
  return (
    <Modal open={open}>
      <h2>Cancel Session: {sessionName}?</h2>
      {hasUncommittedChanges && (
        <Warning>This session has uncommitted changes that will be lost!</Warning>
      )}
      <button onClick={() => onConfirm(hasUncommittedChanges)}>
        {hasUncommittedChanges ? 'Force Cancel' : 'Cancel Session'}
      </button>
      <button onClick={onCancel}>Keep Session</button>
    </Modal>
  )
}
```

### 4. Button Implementation in Sidebar

```typescript
// In Sidebar.tsx session item
<div className="flex items-center gap-1 opacity-0 group-hover:opacity-100">
  <button 
    onClick={(e) => {
      e.stopPropagation() // Prevent session selection
      handleFinishSession(s.info.session_id, s.info.session_id, s.info.worktree_path)
    }}
    className="text-[11px] px-2 py-0.5 rounded bg-green-800/60 hover:bg-green-700/60"
  >
    Finish
  </button>
  <button 
    onClick={(e) => {
      e.stopPropagation()
      handleCancelSession(s.info.session_id, s.info.session_id, s.info.has_uncommitted_changes)
    }}
    className="text-[11px] px-2 py-0.5 rounded bg-red-800/60 hover:bg-red-700/60"
  >
    Cancel
  </button>
</div>
```

## Implementation Steps

### Phase 1: Backend Commands
1. Add `finish_session` and `cancel_session` methods to ParaCliClient
2. Add corresponding Tauri commands in main.rs
3. Test commands with mock data

### Phase 2: Modal Components
1. Create FinishSessionModal component with commit message input
2. Create CancelConfirmation component with warning for uncommitted changes
3. Add modal state management to App.tsx

### Phase 3: Event System
1. Define SessionActionEvent interface
2. Add event listener in App.tsx
3. Dispatch events from Sidebar buttons

### Phase 4: Integration
1. Wire up modals to Tauri commands
2. Handle success/error responses
3. Refresh session list after actions
4. Show notifications for success/failure

### Phase 5: Polish
1. Add loading states during operations
2. Keyboard shortcuts (Cmd+Shift+F for finish, Cmd+Shift+X for cancel)
3. Disable buttons for inappropriate states (e.g., can't finish session with no changes)
4. Add tooltips explaining what each action does

## Edge Cases to Handle

1. **Session with no changes**: Disable finish button or show warning
2. **Uncommitted changes**: Show warning in cancel dialog, require force flag
3. **Currently selected session**: Switch to orchestrator after cancel/finish
4. **Active terminals**: Clean up terminals when session is cancelled
5. **Network/CLI failures**: Show error messages, allow retry
6. **Concurrent operations**: Disable buttons during operations

## Alternative Approaches Considered

### Context API Approach
- Pro: Cleaner component props
- Con: More boilerplate, overkill for simple actions

### Direct Props Drilling
- Pro: Explicit data flow
- Con: Props need to pass through multiple levels

### State Management (Redux/Zustand)
- Pro: Centralized state
- Con: Additional dependency, complexity for simple feature

## Recommended Approach

The **Event-Driven Approach** is recommended because:
1. Minimal coupling between components
2. Easy to add actions from any component
3. Consistent with existing event patterns in the app
4. Simple to implement and test
5. Natural place (App.tsx) for global action handling

## Security Considerations

1. **Confirmation dialogs**: Always confirm destructive actions
2. **Force flag handling**: Only use force when explicitly confirmed by user
3. **Error messages**: Don't expose sensitive paths in error messages
4. **Input validation**: Validate commit messages (not empty, reasonable length)

## Testing Strategy

1. **Unit tests**: Test individual command functions
2. **Integration tests**: Test full flow from button click to CLI execution
3. **Error scenarios**: Test network failures, CLI errors, invalid inputs
4. **UI tests**: Test modal interactions, button states

## Future Enhancements

1. **Batch operations**: Select multiple sessions to cancel
2. **PR creation**: Add button to create PR directly (using gh CLI)
3. **Undo cancel**: Recover recently cancelled session
4. **Session templates**: Save commit message templates
5. **Diff preview**: Show changes before finishing session
6. **Auto-commit**: Option to auto-commit before finishing
# Para Web UI Integration Validation

## Current Para CLI Analysis

### ❌ Missing JSON Output Support
**Finding**: Para CLI does NOT currently support `--json` flag for machine-readable output.
- `para list` only outputs human-readable format
- No structured data export available

**Required CLI Change**: Add JSON output support to `para list` command

### ✅ Session Management Commands Work
- `para start [name] -p "prompt"` - Creates new sessions
- `para list` - Shows active sessions (text only)
- `para monitor` - Interactive TUI (not suitable for web embedding)
- `para cancel [session]` - Removes sessions
- `para finish "message"` - Completes sessions

### ✅ Terminal Integration Approach Valid
- Can spawn `para resume <session>` in PTY for Claude terminal
- Can spawn shell in worktree directory for user terminal
- Can capture `para monitor` output (though it's a TUI)

## Required CLI Changes

### 1. Add JSON Output to List Command
```rust
// Add to src/cli/commands/list/mod.rs
#[derive(Args)]
pub struct ListArgs {
    #[arg(long, short = 'j', help = "Output in JSON format")]
    json: bool,
    // ... existing args
}
```

Expected JSON format:
```json
{
  "sessions": [
    {
      "name": "feature-auth",
      "branch": "para/feature-auth",
      "worktree_path": "/path/to/.para/worktrees/feature-auth",
      "status": "active",
      "has_changes": true,
      "last_modified": "2025-07-31T21:35:42Z",
      "base_branch": "main",
      "task_description": "implement user authentication"
    }
  ]
}
```

### 2. Add Machine-Readable Status Command
```bash
para status --json [session-name]
```
Returns detailed session info including git stats, file changes, etc.

## Project Management Strategy

### Current Para Limitation
Para operates in the current directory only - it has no concept of "projects" or managing multiple repositories.

### Web UI Project Management Approach

1. **Project Storage** (Web UI maintains this):
```json
{
  "projects": [
    {
      "id": "uuid",
      "name": "My App",
      "path": "/Users/me/repos/my-app",
      "last_opened": "2025-07-31T21:35:42Z"
    }
  ]
}
```

2. **Session Retrieval Per Project**:
```javascript
// Web UI server pseudocode
async function getProjectSessions(projectPath) {
  // Change to project directory
  process.chdir(projectPath);
  
  // Run para list --json
  const output = await exec('para list --json');
  return JSON.parse(output);
}
```

3. **Starting New Sessions**:
```javascript
async function startSession(projectPath, name, prompt) {
  process.chdir(projectPath);
  await exec(`para start ${name} -p "${prompt}"`);
}
```

## Terminal Communication Architecture

### Valid Approach Confirmed:
```
[Browser XTerm.js] <--WebSocket--> [Node.js Server] <--PTY--> [Para CLI Process]
```

### Terminal Types and Commands:

1. **Claude Terminal** (Interactive):
```javascript
// Spawn in worktree directory
const pty = spawn('bash', ['-c', 'para resume ' + sessionName], {
  cwd: session.worktree_path,
  env: process.env
});
```

2. **User Terminal** (Interactive):
```javascript
// Regular shell in worktree
const pty = spawn(process.env.SHELL || 'bash', [], {
  cwd: session.worktree_path,
  env: process.env
});
```

3. **Para Monitor** (Read-only):
```javascript
// Run in project root
const pty = spawn('para', ['monitor'], {
  cwd: projectPath,
  env: { ...process.env, TERM: 'xterm-256color' }
});
```

## IDE Integration

### ✅ Opening Worktree in IDE Works
```javascript
// Get IDE from para config
const config = JSON.parse(await exec('para config show --json')); // Need this too
const ideCommand = config.ide.command; // e.g., 'code' or 'cursor'

// Open worktree
await exec(`${ideCommand} ${session.worktree_path}`);
```

## Summary of Required CLI Changes

### Essential (Blocking):
1. **`para list --json`** - Output sessions in JSON format
2. **`para config show --json`** - Export configuration as JSON

### Nice to Have:
3. **`para status --json [session]`** - Detailed session info
4. **`para project list`** - Multi-project support (future)

## Alternative: Parse Text Output
If CLI changes are not immediately possible, the web UI could parse the text output:
```javascript
// Fragile but works
function parseParaList(output) {
  const sessions = [];
  const lines = output.split('\n');
  // Parse "Session: name" lines
  // Extract worktree paths, etc.
  return sessions;
}
```

However, this is brittle and not recommended for production.

## Recommended Implementation Order

1. **Phase 1**: Add `--json` flag to `para list` command
2. **Phase 2**: Build web UI with project management layer
3. **Phase 3**: Add `para config show --json` for IDE detection
4. **Phase 4**: Enhance with additional JSON endpoints as needed
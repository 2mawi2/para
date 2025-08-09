# Para Web UI Prototype

THINK HARDER: What's the minimal way to create a visual interface for Para that shows multiple Claude sessions, terminals, and session info while reusing existing Para CLI functionality?

## Core Requirement
Build a web-based UI that displays Para sessions in a split-screen layout with embedded Claude terminals, Para monitor overview, and user terminals, allowing visual management of parallel development sessions.

## Observable Behavior
- User opens `http://localhost:3000` → Shows project selector
- User adds project directory → Project appears in list
- User opens project → Shows session list on left
- User clicks session → Center shows Claude terminal, right shows Para monitor + user terminal
- User clicks "Open in IDE" → VS Code/Cursor opens for that worktree
- User drags panel borders → Panels resize accordingly
- All terminals are functional → User can interact with Claude, run commands

## Implementation Boundaries
- **Must Have**: 
  - Add `--json` flag to `para list` command first
  - Add `--json` flag to `para config show` command
  - Web server running on localhost
  - Session list from Para CLI JSON output
  - Embedded terminal for Claude session (xterm.js)
  - Read-only Para monitor terminal
  - Interactive user terminal per worktree
  - Basic panel resizing
  - Open worktree in IDE button
- **Must NOT Have**: 
  - Authentication/multi-user support
  - Remote hosting capabilities
  - Custom Para reimplementation
  - Complex database (use simple JSON file)
  - Electron packaging (in prototype)
- **Can Assume**: 
  - Para CLI is installed and working
  - User has Claude Code + IDE configured
  - Single user on localhost
  - Modern browser with WebSocket support

## Verification Strategy
1. Start web server: `npm run dev` → Server starts on port 3000
2. Open browser → Project selector page loads
3. Add test project → Project appears in list
4. Open project → Sessions load from `para list --json`
5. Click session → Terminals render and are interactive
6. Test all terminals work: Type commands, see output
7. DONE when: All panels show correct content and terminals are interactive

## Technical Context
- Components: 
  - `web-ui/` - New directory for web UI
  - `web-ui/server.js` - Express server with WebSocket support
  - `web-ui/public/` - Frontend HTML/CSS/JS
  - `web-ui/lib/terminal-manager.js` - Spawn and manage PTY sessions
  - `web-ui/lib/para-wrapper.js` - Interface with Para CLI
- Constraints: 
  - Must use existing Para CLI commands
  - Terminal rendering requires xterm.js + node-pty
  - WebSockets needed for terminal communication
  - Cross-origin issues if not on same port
- Patterns: 
  - Use Para's JSON output (`para list --json`)
  - Spawn processes in worktree directories
  - WebSocket per terminal connection

## Development Approach
1. First add JSON support to Para CLI:
   - Add `--json` flag to `list` command
   - Add `--json` flag to `config show` command
2. Start with Express server that calls `para list --json`
3. Create basic HTML with three panels (use CSS Grid)
4. Add xterm.js for terminal rendering
5. Implement WebSocket->PTY bridge for terminals
6. Add panel resizing with CSS resize property
7. Stop when clicking session shows working terminals

## Architecture Decision: Web vs Native

### Web Application (Recommended for Prototype)
**Pros:**
- Quick to prototype with Express + xterm.js
- No installation required beyond `npm install`
- Cross-platform by default
- Easy to iterate and debug
- Can use existing web terminal libraries

**Cons:**
- Requires browser tab open
- Less "native" feel
- Potential security concerns (localhost only mitigates this)

### Electron App (Future Enhancement)
**Pros:**
- Native app experience
- System tray integration possible
- Better OS integration

**Cons:**
- More complex build/distribution
- Code signing requirements
- Larger download size
- Overhead for prototype

**Recommendation:** Start with web app for prototype, can wrap in Electron later if needed.

## Minimal Prototype Structure

```
para/
└── web-ui/
    ├── package.json
    ├── server.js              # Express + WebSocket server
    ├── lib/
    │   ├── terminal-manager.js # PTY management
    │   └── para-wrapper.js    # Para CLI interface
    └── public/
        ├── index.html         # Main UI
        ├── style.css          # Grid layout + styling
        └── app.js             # Frontend logic
```

## Key Implementation Details

### CLI Changes Required First
```rust
// In src/cli/commands/list/mod.rs
#[derive(Args)]
pub struct ListArgs {
    #[arg(long, short = 'j', help = "Output in JSON format")]
    json: bool,
    // ... existing args
}

// In src/cli/commands/config/mod.rs  
#[derive(Args)]
pub struct ShowArgs {
    #[arg(long, short = 'j', help = "Output in JSON format")]
    json: bool,
}
```

### Terminal Management
- Each terminal gets unique WebSocket connection
- Claude terminal: Spawn `para resume <session>` in PTY
- User terminal: Spawn shell in worktree directory  
- Para monitor: Spawn `para monitor` with ANSI color support

### Project Management Layer
```javascript
// Web UI maintains projects.json
{
  "projects": [
    {
      "id": "uuid-here",
      "name": "My Project", 
      "path": "/Users/me/repos/my-project",
      "lastOpened": "2025-07-31T21:35:42Z"
    }
  ]
}

// Get sessions for a project
async function getProjectSessions(projectPath) {
  const { stdout } = await exec('para list --json', { cwd: projectPath });
  return JSON.parse(stdout);
}
```

### Layout System
- CSS Grid for main layout
- Split.js library for resizable panels
- Panels: 25% left (sessions), 50% center (Claude), 25% right (terminals)

### Session Updates
- Poll `para list --json` every 5 seconds
- Update UI when sessions change
- Highlight active/selected session

## MVP User Flow
1. Run `npm start` in web-ui directory
2. Browser opens to http://localhost:3000
3. Click "Add Project" → File picker → Select git repo
4. Project appears → Click to open
5. Left panel shows Para sessions for that project
6. Click session → Center shows Claude, right shows terminals
7. Interact with Claude normally through terminal
8. Click "Open in IDE" → VS Code opens that worktree
# Conductor Application Analysis

## Technology Stack

Conductor is built using **Tauri v2** - a Rust-based framework for building native desktop applications with web frontends.

### Core Technologies:
- **Backend**: Rust + Tauri
- **Terminal Support**: `tauri-plugin-pty` 
- **Frontend**: Web technologies (HTML/JS/CSS)
- **Database**: SQLite via `tauri-plugin-sql`
- **Other Plugins**: 
  - File system access (`tauri-plugin-fs`)
  - HTTP requests (`tauri-plugin-http`)
  - Shell commands (`tauri-plugin-shell`)
  - Notifications, dialogs, deep links, etc.

## How Terminal Embedding Works

### 1. **PTY Plugin Architecture**
```
[Frontend xterm.js] <--IPC--> [Tauri Rust] <--PTY--> [Native Shell Process]
```

### 2. **Implementation Details**

**Rust Side**:
- Uses `portable-pty` crate for cross-platform PTY support
- Spawns native shell processes (bash, zsh, etc.)
- Manages PTY lifecycle and data streaming

**Frontend Side**:
- Uses `xterm.js` for terminal rendering
- Bidirectional data flow via Tauri's IPC system
- WebSocket-like communication for real-time updates

### 3. **Key Code Patterns**
```javascript
// Frontend terminal setup
const pty = spawn("bash", [], {
  cols: term.cols,
  rows: term.rows,
  cwd: sessionPath
})

// Data flow
pty.onData(data => term.write(data))  // Shell → Terminal
term.onData(data => pty.write(data))  // Terminal → Shell
```

## Advantages Over Web-Only Solution

1. **Native Terminal Support** - Real PTY, not simulated
2. **Full System Access** - Can run any command/binary
3. **Performance** - Direct process communication
4. **Security** - No web server exposure
5. **Small Bundle** - ~20MB vs Electron's 100MB+

## How This Applies to Para UI

### Recommended Architecture for Para:

```
Para UI (Tauri App)
├── Backend (Rust)
│   ├── Para CLI Integration
│   ├── PTY Management 
│   └── Session State
└── Frontend (Web)
    ├── Session List (Left Panel)
    ├── Claude Terminal (Center)
    ├── Git/Files View (Top Right)
    └── User Terminal (Bottom Right)
```

### Implementation Plan:

1. **Create Tauri App** with para as dependency
2. **Add tauri-plugin-pty** for terminal support
3. **Frontend Layout** with resizable panels
4. **Terminal Integration**:
   - Claude terminal: `spawn("para", ["resume", sessionName])`
   - User terminal: `spawn(shellPath, [], { cwd: worktreePath })`
   - Para monitor: `spawn("para", ["monitor"])`

### Key Benefits:
- ✅ Native terminals work perfectly
- ✅ Can reuse Para's Rust code
- ✅ Cross-platform (macOS, Windows, Linux)
- ✅ Modern UI with web technologies
- ✅ Small, fast native app

### Challenges:
- Learning curve for Tauri
- Need to handle PTY lifecycle carefully
- Terminal resize handling
- Proper cleanup on exit

## Conclusion

Conductor proves that Tauri + PTY plugin is an excellent solution for building terminal-based developer tools. It provides the perfect balance of:
- Native performance and capabilities
- Modern web UI flexibility
- Small bundle size
- Cross-platform support

This is the ideal approach for Para UI, much better than:
- TUI (too limited for rich UI)
- Web-only (can't embed real terminals)
- Electron (too heavy)
- Native macOS (platform locked)
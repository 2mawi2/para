# Para UI Evaluation: Terminal TUI vs Native App

## Critical Requirements
1. **Native terminal embedding** - Must run real terminals with full PTY support
2. **Multiple terminal views** - Claude session, user terminal, Para monitor
3. **Git integration** - Show diffs, file changes, commit history
4. **File tree navigation** - Browse worktree files
5. **IDE launching** - Open sessions in VS Code/Cursor
6. **Cross-platform potential** - Initially macOS, later Windows

## Option 1: Terminal TUI Application (like lazygit)

### Technology Stack
- **Language**: Rust (same as Para)
- **TUI Framework**: Ratatui (already used in Para monitor)
- **Terminal Multiplexing**: Custom PTY management or integrate with tmux

### Pros
- ✅ **Native terminal support** - Full PTY control, no limitations
- ✅ **Consistent with Para** - Same tech stack, can share code
- ✅ **Lightweight** - No GUI dependencies, works over SSH
- ✅ **Keyboard-driven** - Power user friendly
- ✅ **Easy distribution** - Single binary like Para itself

### Cons
- ❌ **Limited UI flexibility** - Text-only constraints
- ❌ **No mouse-driven resize** - Fixed layouts or keyboard shortcuts only
- ❌ **Terminal inception issues** - Running terminals inside terminals can be tricky
- ❌ **Learning curve** - Users need to learn keybindings

### Implementation Approach
```rust
// Extend existing Para monitor with new views
pub enum View {
    SessionList,      // Current para monitor
    GitChanges,       // Git diff view
    FileTree,         // File browser
    TerminalGrid,     // Multiple embedded terminals
}

// Use tmux or direct PTY management
pub struct TerminalPane {
    pty: Pty,
    buffer: Vec<String>,
    cursor: (u16, u16),
}
```

### Risk Assessment
- **Medium Risk**: Terminal-in-terminal complexity
- **Low Risk**: Technology is proven (lazygit, k9s, etc.)
- **Challenge**: Smooth terminal switching/scrolling

## Option 2: Native macOS Application

### Technology Stack
- **Language**: Swift + SwiftUI
- **Terminal**: Embed Terminal.app views or custom NSTextView
- **Framework**: AppKit for terminal embedding

### Pros
- ✅ **Native look and feel** - Proper macOS app
- ✅ **Rich UI possibilities** - Tabs, splits, floating panels
- ✅ **Native terminal embedding** - Can use NSTask + PTY
- ✅ **Mouse support** - Drag to resize, click to navigate
- ✅ **System integration** - Notifications, dock, menu bar

### Cons
- ❌ **Platform locked** - macOS only initially
- ❌ **Different tech stack** - Can't share code with Para
- ❌ **App Store complications** - Signing, notarization
- ❌ **Heavier resource usage** - Full GUI app
- ❌ **Windows port difficult** - Would need complete rewrite

### Implementation Approach
```swift
// SwiftUI view with embedded terminals
struct ParaSessionView: View {
    @State var sessions: [ParaSession]
    
    var body: some View {
        HSplitView {
            SessionList(sessions: $sessions)
            VSplitView {
                TerminalView(session: selectedSession)
                HStack {
                    GitDiffView()
                    UserTerminalView()
                }
            }
        }
    }
}

// Terminal embedding using NSViewRepresentable
struct TerminalView: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let task = Process()
        task.launchPath = "/usr/bin/env"
        task.arguments = ["para", "resume", session.name]
        // Set up PTY...
    }
}
```

### Risk Assessment
- **High Risk**: Platform-specific code
- **Medium Risk**: Terminal embedding complexity
- **Low Risk**: Well-documented Apple APIs

## Option 3: Electron + Native Modules

### Technology Stack
- **Framework**: Electron with node-pty
- **Frontend**: React/Vue + xterm.js
- **Backend**: Node.js native modules for PTY

### Pros
- ✅ **Cross-platform** - Same code for macOS/Windows/Linux
- ✅ **Web technologies** - Familiar to many developers
- ✅ **Rich UI** - Full HTML/CSS capabilities
- ✅ **Terminal support** - node-pty provides real PTY

### Cons
- ❌ **Heavy bundle size** - 100MB+ for Chromium
- ❌ **Performance overhead** - Chromium + Node.js
- ❌ **Not truly native** - Doesn't feel like macOS app
- ❌ **Security concerns** - Running terminals in web context

### Risk Assessment
- **Low Risk**: Proven tech (VS Code uses this)
- **Medium Risk**: Performance with multiple terminals
- **High Risk**: Security sandboxing complexities

## Option 4: Tauri (Rust + Web UI)

### Technology Stack
- **Backend**: Rust (can reuse Para code)
- **Frontend**: HTML/JS with native OS webview
- **Terminal**: Custom Rust PTY bridge

### Pros
- ✅ **Lightweight** - 10MB vs Electron's 100MB+
- ✅ **Native performance** - Rust backend
- ✅ **Cross-platform** - Single codebase
- ✅ **Secure** - Better isolation than Electron

### Cons
- ❌ **Newer technology** - Less proven
- ❌ **Terminal embedding harder** - Need custom bridge
- ❌ **Webview limitations** - OS-dependent rendering

## Recommendation: Terminal TUI (Enhanced Para Monitor)

### Why TUI is the Best Fit:

1. **Consistency** - Extends existing Para monitor
2. **Distribution** - Single binary, no installation complexity
3. **Performance** - Minimal resource usage
4. **Para Philosophy** - Stays true to CLI-first approach
5. **Quick Development** - Can reuse existing code

### Proposed Architecture:

```
┌─────────────────────────────────────────────────────┐
│  Para UI (Enhanced Monitor)                         │
├──────────┬────────────────────┬────────────────────┤
│ Sessions │ Claude Terminal    │ Git/Files          │
│  List    │                    ├────────────────────┤
│          │                    │ User Terminal      │
│ [>] auth │ $ para resume auth │                    │
│ [ ] api  │ Claude> working... │ $ npm test         │
│ [ ] ui   │                    │ ...                │
└──────────┴────────────────────┴────────────────────┘
[F1]Help [F2]Git [F3]Files [F4]Term [q]Quit [o]Open IDE
```

### Implementation Plan:

1. **Phase 1**: Extend para monitor with pane management
2. **Phase 2**: Add terminal embedding (start with single terminal)
3. **Phase 3**: Multiple terminal support
4. **Phase 4**: Git diff viewer
5. **Phase 5**: File tree browser

### Fallback Option:

If TUI proves too limiting, **Tauri** would be the next best choice:
- Allows rich UI while keeping Rust core
- Can gradually migrate from TUI to GUI
- Maintains cross-platform compatibility

### Critical Success Factors:

1. **Terminal Management**: Use `portable-pty` crate for cross-platform PTY
2. **Layout System**: Implement flexible pane splitting (like tmux)
3. **Performance**: Efficient terminal buffer management
4. **Keybindings**: Vim-like navigation for power users

This approach aligns with Para's philosophy while solving the native terminal requirement.
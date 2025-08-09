# Para Tauri UI - Technical Implementation Details

## Overview
Build a native desktop application for Para using Tauri + PTY plugin, enabling multiple embedded terminals for Claude sessions, user terminals, and Para monitoring.

## Step 1: Project Setup

### 1.1 Create Tauri Project Structure
```bash
# Create new Tauri app
npm create tauri-app@latest para-ui -- --template vanilla

# Project structure
para-ui/
├── src/                    # Frontend (HTML/JS/CSS)
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/              # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   └── main.rs
│   └── tauri.conf.json
└── package.json
```

### 1.2 Add Dependencies

**Cargo.toml** (Rust dependencies):
```toml
[dependencies]
tauri = { version = "2.0", features = [] }
tauri-plugin-pty = "0.1"
tauri-plugin-fs = "2.0"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Add Para as a library dependency
para = { path = "../para" }  # Or version if published

[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]
tauri-plugin-shell = "2.0"
```

**package.json** (Frontend dependencies):
```json
{
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "xterm": "^5.3.0",
    "xterm-addon-fit": "^0.8.0",
    "xterm-addon-web-links": "^0.9.0",
    "split.js": "^1.6.5"
  }
}
```

## Step 2: Rust Backend Implementation

### 2.1 Main Application Setup
```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_plugin_pty::PtyPlugin;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
struct ParaSession {
    name: String,
    branch: String,
    worktree_path: String,
    status: String,
    task_description: Option<String>,
}

#[derive(Default)]
struct AppState {
    projects: Mutex<HashMap<String, String>>, // project_id -> path
    active_project: Mutex<Option<String>>,
}

#[tauri::command]
async fn get_para_sessions(project_path: String) -> Result<Vec<ParaSession>, String> {
    // Change to project directory and run para list --json
    std::env::set_current_dir(&project_path)
        .map_err(|e| format!("Failed to change directory: {}", e))?;
    
    // TODO: Once para supports --json, parse the output
    // For now, parse text output or use Para as a library
    let output = std::process::Command::new("para")
        .arg("list")
        .arg("--verbose")
        .output()
        .map_err(|e| format!("Failed to run para list: {}", e))?;
    
    // Parse output and return sessions
    parse_para_list_output(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
async fn start_para_session(
    project_path: String,
    name: String,
    prompt: String
) -> Result<String, String> {
    std::env::set_current_dir(&project_path)
        .map_err(|e| format!("Failed to change directory: {}", e))?;
    
    let output = std::process::Command::new("para")
        .arg("start")
        .arg(&name)
        .arg("-p")
        .arg(&prompt)
        .arg("--dangerously-skip-permissions")
        .output()
        .map_err(|e| format!("Failed to start para session: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Started session: {}", name))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn open_in_ide(worktree_path: String) -> Result<(), String> {
    // Get IDE from para config
    let config_output = std::process::Command::new("para")
        .arg("config")
        .arg("show")
        .arg("--json") // Once implemented
        .output()
        .map_err(|e| format!("Failed to get para config: {}", e))?;
    
    // Parse IDE command and launch
    // For now, assume 'code' or 'cursor'
    std::process::Command::new("code")
        .arg(&worktree_path)
        .spawn()
        .map_err(|e| format!("Failed to open IDE: {}", e))?;
    
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_pty::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_para_sessions,
            start_para_session,
            open_in_ide,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn parse_para_list_output(output: String) -> Result<Vec<ParaSession>, String> {
    // TODO: Implement parsing logic
    // This is temporary until para supports --json
    let mut sessions = Vec::new();
    
    // Parse the verbose output format
    // Session: name
    //   Status: ...
    //   Branch: ...
    //   Worktree: ...
    
    Ok(sessions)
}
```

### 2.2 PTY Management Layer
```rust
// src-tauri/src/pty_manager.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

pub struct PtyManager {
    terminals: Arc<Mutex<HashMap<String, PtyHandle>>>,
}

pub struct PtyHandle {
    id: String,
    pty_id: u32, // From tauri-plugin-pty
    session_name: String,
    terminal_type: TerminalType,
}

#[derive(Debug, Clone)]
pub enum TerminalType {
    ClaudeSession { session_name: String },
    UserTerminal { working_dir: String },
    ParaMonitor,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn create_claude_terminal(
        &self,
        app: AppHandle,
        session_name: String,
        worktree_path: String,
    ) -> Result<String, String> {
        // Use tauri-plugin-pty to spawn para resume
        let pty_id = spawn_pty(
            app,
            "para",
            vec!["resume", &session_name],
            Some(&worktree_path),
        ).await?;
        
        let terminal_id = format!("claude-{}", uuid::Uuid::new_v4());
        let handle = PtyHandle {
            id: terminal_id.clone(),
            pty_id,
            session_name: session_name.clone(),
            terminal_type: TerminalType::ClaudeSession { session_name },
        };
        
        self.terminals.lock().unwrap().insert(terminal_id.clone(), handle);
        Ok(terminal_id)
    }
    
    pub async fn create_user_terminal(
        &self,
        app: AppHandle,
        working_dir: String,
    ) -> Result<String, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        
        let pty_id = spawn_pty(
            app,
            &shell,
            vec![],
            Some(&working_dir),
        ).await?;
        
        let terminal_id = format!("user-{}", uuid::Uuid::new_v4());
        let handle = PtyHandle {
            id: terminal_id.clone(),
            pty_id,
            session_name: String::new(),
            terminal_type: TerminalType::UserTerminal { working_dir },
        };
        
        self.terminals.lock().unwrap().insert(terminal_id.clone(), handle);
        Ok(terminal_id)
    }
}
```

## Step 3: Frontend Implementation

### 3.1 HTML Structure
```html
<!-- src/index.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Para UI</title>
    <link rel="stylesheet" href="https://unpkg.com/xterm/css/xterm.css" />
    <link rel="stylesheet" href="styles.css" />
</head>
<body>
    <div id="app">
        <!-- Project Selector -->
        <div id="project-bar">
            <select id="project-selector">
                <option value="">Select Project...</option>
            </select>
            <button id="add-project">Add Project</button>
        </div>
        
        <!-- Main Layout -->
        <div id="main-layout">
            <!-- Left Panel: Session List -->
            <div id="session-panel">
                <div class="panel-header">
                    <h3>Sessions</h3>
                    <button id="new-session">+</button>
                </div>
                <div id="session-list"></div>
            </div>
            
            <!-- Center Panel: Claude Terminal -->
            <div id="center-panel">
                <div class="panel-header">
                    <h3 id="claude-session-name">Select a session</h3>
                    <button id="open-ide">Open in IDE</button>
                </div>
                <div id="claude-terminal"></div>
            </div>
            
            <!-- Right Panel: Split between Git/Files and User Terminal -->
            <div id="right-panel">
                <div id="top-right">
                    <div class="panel-header">
                        <h3>Git Status</h3>
                    </div>
                    <div id="git-status"></div>
                </div>
                <div id="bottom-right">
                    <div class="panel-header">
                        <h3>Terminal</h3>
                    </div>
                    <div id="user-terminal"></div>
                </div>
            </div>
        </div>
    </div>
    
    <script src="main.js" type="module"></script>
</body>
</html>
```

### 3.2 JavaScript Terminal Integration
```javascript
// src/main.js
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { spawn, write, resize, kill } from '@tauri-apps/plugin-pty';
import { invoke } from '@tauri-apps/api/core';
import Split from 'split.js';

class ParaUI {
    constructor() {
        this.terminals = new Map();
        this.currentSession = null;
        this.currentProject = null;
        
        this.initializeUI();
        this.setupEventListeners();
    }
    
    initializeUI() {
        // Create split panels
        Split(['#session-panel', '#center-panel', '#right-panel'], {
            sizes: [20, 50, 30],
            minSize: [150, 300, 200],
            gutterSize: 5,
        });
        
        Split(['#top-right', '#bottom-right'], {
            direction: 'vertical',
            sizes: [50, 50],
            minSize: [100, 100],
            gutterSize: 5,
        });
    }
    
    async createClaudeTerminal(sessionName, worktreePath) {
        // Clean up existing terminal if any
        if (this.terminals.has('claude')) {
            this.cleanupTerminal('claude');
        }
        
        // Create xterm.js instance
        const term = new Terminal({
            fontFamily: 'JetBrains Mono, Menlo, monospace',
            fontSize: 14,
            theme: {
                background: '#1e1e1e',
                foreground: '#cccccc',
            },
            cursorBlink: true,
        });
        
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        term.loadAddon(new WebLinksAddon());
        
        // Open terminal in DOM
        const container = document.getElementById('claude-terminal');
        container.innerHTML = '';
        term.open(container);
        fitAddon.fit();
        
        // Spawn PTY process for para resume
        const pty = await spawn('para', ['resume', sessionName], {
            cwd: worktreePath,
            env: {},
            cols: term.cols,
            rows: term.rows,
        });
        
        // Setup bidirectional communication
        pty.onData((data) => {
            term.write(data);
        });
        
        term.onData((data) => {
            write(pty.pid, data);
        });
        
        // Handle resize
        term.onResize(({ cols, rows }) => {
            resize(pty.pid, cols, rows);
        });
        
        // Store terminal info
        this.terminals.set('claude', {
            terminal: term,
            pty: pty,
            fitAddon: fitAddon,
        });
        
        // Update UI
        document.getElementById('claude-session-name').textContent = sessionName;
    }
    
    async createUserTerminal(workingDir) {
        if (this.terminals.has('user')) {
            this.cleanupTerminal('user');
        }
        
        const term = new Terminal({
            fontFamily: 'JetBrains Mono, Menlo, monospace',
            fontSize: 14,
            theme: {
                background: '#0c0c0c',
                foreground: '#cccccc',
            },
            cursorBlink: true,
        });
        
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        
        const container = document.getElementById('user-terminal');
        container.innerHTML = '';
        term.open(container);
        fitAddon.fit();
        
        // Spawn user's default shell
        const shell = await invoke('get_user_shell');
        const pty = await spawn(shell, [], {
            cwd: workingDir,
            env: {},
            cols: term.cols,
            rows: term.rows,
        });
        
        pty.onData((data) => {
            term.write(data);
        });
        
        term.onData((data) => {
            write(pty.pid, data);
        });
        
        term.onResize(({ cols, rows }) => {
            resize(pty.pid, cols, rows);
        });
        
        this.terminals.set('user', {
            terminal: term,
            pty: pty,
            fitAddon: fitAddon,
        });
    }
    
    async loadSessions() {
        if (!this.currentProject) return;
        
        try {
            const sessions = await invoke('get_para_sessions', {
                projectPath: this.currentProject
            });
            
            this.renderSessionList(sessions);
        } catch (error) {
            console.error('Failed to load sessions:', error);
        }
    }
    
    renderSessionList(sessions) {
        const listEl = document.getElementById('session-list');
        listEl.innerHTML = '';
        
        sessions.forEach(session => {
            const item = document.createElement('div');
            item.className = 'session-item';
            item.innerHTML = `
                <div class="session-name">${session.name}</div>
                <div class="session-status">${session.status}</div>
                <div class="session-task">${session.task_description || ''}</div>
            `;
            
            item.addEventListener('click', () => {
                this.selectSession(session);
            });
            
            listEl.appendChild(item);
        });
    }
    
    async selectSession(session) {
        this.currentSession = session;
        
        // Update active state in UI
        document.querySelectorAll('.session-item').forEach(el => {
            el.classList.remove('active');
        });
        event.currentTarget.classList.add('active');
        
        // Create terminals for this session
        await this.createClaudeTerminal(session.name, session.worktree_path);
        await this.createUserTerminal(session.worktree_path);
        
        // Load git status
        await this.loadGitStatus(session.worktree_path);
    }
    
    async loadGitStatus(worktreePath) {
        // Run git status in the worktree
        const gitStatus = await invoke('get_git_status', {
            path: worktreePath
        });
        
        document.getElementById('git-status').innerHTML = `<pre>${gitStatus}</pre>`;
    }
    
    cleanupTerminal(id) {
        const termInfo = this.terminals.get(id);
        if (termInfo) {
            kill(termInfo.pty.pid);
            termInfo.terminal.dispose();
            this.terminals.delete(id);
        }
    }
    
    setupEventListeners() {
        // Add project button
        document.getElementById('add-project').addEventListener('click', async () => {
            const path = await invoke('select_project_directory');
            if (path) {
                await this.addProject(path);
            }
        });
        
        // New session button
        document.getElementById('new-session').addEventListener('click', async () => {
            const name = prompt('Session name:');
            const prompt = prompt('Task description:');
            
            if (name && prompt) {
                await invoke('start_para_session', {
                    projectPath: this.currentProject,
                    name,
                    prompt
                });
                
                await this.loadSessions();
            }
        });
        
        // Open in IDE button
        document.getElementById('open-ide').addEventListener('click', async () => {
            if (this.currentSession) {
                await invoke('open_in_ide', {
                    worktreePath: this.currentSession.worktree_path
                });
            }
        });
        
        // Handle window resize
        window.addEventListener('resize', () => {
            this.terminals.forEach(termInfo => {
                termInfo.fitAddon.fit();
            });
        });
    }
}

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    new ParaUI();
});
```

### 3.3 CSS Styling
```css
/* src/styles.css */
body {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #1a1a1a;
    color: #e0e0e0;
}

#app {
    display: flex;
    flex-direction: column;
    height: 100vh;
}

#project-bar {
    display: flex;
    align-items: center;
    padding: 10px;
    background: #2a2a2a;
    border-bottom: 1px solid #3a3a3a;
}

#main-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
}

#session-panel {
    background: #252525;
    border-right: 1px solid #3a3a3a;
    display: flex;
    flex-direction: column;
}

.panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    background: #2a2a2a;
    border-bottom: 1px solid #3a3a3a;
}

#session-list {
    flex: 1;
    overflow-y: auto;
}

.session-item {
    padding: 10px;
    border-bottom: 1px solid #3a3a3a;
    cursor: pointer;
}

.session-item:hover {
    background: #2a2a2a;
}

.session-item.active {
    background: #3a3a3a;
}

.session-name {
    font-weight: bold;
    margin-bottom: 4px;
}

.session-status {
    font-size: 12px;
    color: #888;
}

.session-task {
    font-size: 12px;
    color: #aaa;
    margin-top: 4px;
}

#center-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
}

#claude-terminal {
    flex: 1;
    background: #1e1e1e;
}

#right-panel {
    display: flex;
    flex-direction: column;
}

#top-right, #bottom-right {
    display: flex;
    flex-direction: column;
}

#git-status {
    flex: 1;
    padding: 10px;
    overflow-y: auto;
    background: #1e1e1e;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
}

#user-terminal {
    flex: 1;
    background: #0c0c0c;
}

.gutter {
    background-color: #3a3a3a;
    background-repeat: no-repeat;
    background-position: 50%;
}

.gutter.gutter-horizontal {
    cursor: col-resize;
}

.gutter.gutter-vertical {
    cursor: row-resize;
}
```

## Step 4: Key Integration Points

### 4.1 Para CLI JSON Support (Required)
```rust
// In Para's codebase - add JSON output support
#[derive(Serialize)]
struct ListOutput {
    sessions: Vec<SessionInfo>,
}

#[derive(Serialize)]
struct SessionInfo {
    name: String,
    branch: String,
    worktree_path: PathBuf,
    status: SessionStatus,
    task_description: Option<String>,
    git_stats: Option<GitStats>,
}

// Add --json flag to list command
if args.json {
    let output = ListOutput { sessions };
    println!("{}", serde_json::to_string_pretty(&output)?);
} else {
    // Existing human-readable output
}
```

### 4.2 Terminal Lifecycle Management
```javascript
// Proper cleanup on app close
window.addEventListener('beforeunload', () => {
    // Kill all PTY processes
    app.terminals.forEach((termInfo, id) => {
        app.cleanupTerminal(id);
    });
});

// Handle terminal exit
pty.onExit((code) => {
    console.log(`Terminal exited with code ${code}`);
    // Remove from UI or restart
});
```

### 4.3 Project Management
```rust
// Store projects in app data directory
#[tauri::command]
async fn save_projects(state: State<'_, AppState>, projects: Vec<Project>) -> Result<(), String> {
    let app_dir = app_handle.path_resolver().app_data_dir()
        .ok_or("Failed to get app data dir")?;
    
    let projects_file = app_dir.join("projects.json");
    let json = serde_json::to_string_pretty(&projects)
        .map_err(|e| e.to_string())?;
    
    std::fs::write(projects_file, json)
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
```

## Step 5: Build and Distribution

### 5.1 Build Configuration
```json
// tauri.conf.json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devPath": "../src",
    "distDir": "../src"
  },
  "package": {
    "productName": "Para UI",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "dialog": {
        "open": true
      }
    },
    "bundle": {
      "active": true,
      "category": "DeveloperTool",
      "copyright": "",
      "deb": {
        "depends": []
      },
      "externalBin": [],
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.para.ui",
      "longDescription": "",
      "macOS": {
        "entitlements": null,
        "exceptionDomain": "",
        "frameworks": [],
        "providerShortName": null,
        "signingIdentity": null
      },
      "resources": [],
      "shortDescription": "",
      "targets": "all",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      }
    }
  }
}
```

### 5.2 Build Commands
```bash
# Development
npm run tauri dev

# Build for production
npm run tauri build

# Output locations
# macOS: src-tauri/target/release/bundle/dmg/
# Windows: src-tauri/target/release/bundle/msi/
# Linux: src-tauri/target/release/bundle/deb/
```

## Key Technical Considerations

### 1. **Terminal Performance**
- Use virtual scrolling for long outputs
- Throttle terminal updates if needed
- Clean up closed terminals promptly

### 2. **Process Management**
- Track all spawned processes
- Ensure proper cleanup on exit
- Handle terminal crashes gracefully

### 3. **State Synchronization**
- Poll Para for session updates
- Watch for file system changes
- Update UI when sessions change

### 4. **Security**
- Validate all paths before use
- Sanitize terminal input/output
- Use Tauri's permission system

### 5. **Cross-Platform**
- Test shell detection on all platforms
- Handle path separators correctly
- Account for different terminal behaviors

This implementation provides a solid foundation for a Para UI that matches Conductor's capabilities while being specifically tailored for Para's workflow.
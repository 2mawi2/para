# Task 19: Enhanced Shell Completion with Context Awareness

## Objective
Implement advanced shell completion system with context-aware suggestions for session names, branch names, file paths, and command-specific arguments as specified in PRD Section 5.4.

## Background
The current Rust implementation has basic shell completion, but lacks the context-aware features that make para highly productive to use. The PRD specifies enhanced completion features that should provide intelligent suggestions based on the current context and command being typed.

## Requirements

### 1. Context-Aware Completion Engine
Implement intelligent completion that understands:
- Current command context
- Available sessions and their states
- Git branch names and patterns
- File paths for `--file` flags
- Configuration options
- Integration strategies

```rust
// src/cli/completion/mod.rs
pub mod context;
pub mod dynamic;
pub mod generators;
pub mod cache;

use clap::Command;

#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub command: String,
    pub subcommand: Option<String>,
    pub current_arg: String,
    pub previous_args: Vec<String>,
    pub current_directory: PathBuf,
    pub repository_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CompletionSuggestion {
    pub value: String,
    pub description: Option<String>,
    pub completion_type: CompletionType,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub enum CompletionType {
    SessionName,
    BranchName,
    FilePath,
    Flag,
    Subcommand,
    ConfigOption,
    IntegrationStrategy,
}
```

### 2. Session Name Completion
Implement intelligent session name completion:

```rust
// src/cli/completion/dynamic.rs
pub struct SessionCompleter {
    state_dir: PathBuf,
    config: Config,
}

impl SessionCompleter {
    pub fn complete_session_names(&self, prefix: &str) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Active sessions
        for session in self.list_active_sessions()? {
            if session.id.starts_with(prefix) || session.name.starts_with(prefix) {
                suggestions.push(CompletionSuggestion {
                    value: session.name.clone(),
                    description: Some(format!("Active session ({})", session.status)),
                    completion_type: CompletionType::SessionName,
                    priority: match session.status {
                        SessionStatus::Active => 10,
                        SessionStatus::Modified => 9,
                        _ => 8,
                    },
                });
            }
        }
        
        // Archived sessions (lower priority)
        for archived in self.list_archived_sessions()? {
            if archived.starts_with(prefix) {
                suggestions.push(CompletionSuggestion {
                    value: archived,
                    description: Some("Archived session".to_string()),
                    completion_type: CompletionType::SessionName,
                    priority: 5,
                });
            }
        }
        
        // Sort by priority and relevance
        suggestions.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.value.len().cmp(&b.value.len()))
        });
        
        Ok(suggestions)
    }
    
    pub fn complete_for_command(&self, command: &str, prefix: &str) -> Result<Vec<CompletionSuggestion>> {
        match command {
            "cancel" | "resume" => {
                // Only active sessions
                self.complete_active_sessions(prefix)
            },
            "recover" => {
                // Only archived sessions
                self.complete_archived_sessions(prefix)
            },
            "finish" | "integrate" => {
                // Auto-detect current session or allow manual selection
                let mut suggestions = self.complete_active_sessions(prefix)?;
                if let Ok(current) = self.auto_detect_current_session() {
                    suggestions.insert(0, CompletionSuggestion {
                        value: current.name,
                        description: Some("Current session (auto-detected)".to_string()),
                        completion_type: CompletionType::SessionName,
                        priority: 15,
                    });
                }
                Ok(suggestions)
            },
            _ => self.complete_session_names(prefix),
        }
    }
}
```

### 3. Branch Name Completion
Implement Git branch completion for `--branch` flags:

```rust
// src/cli/completion/dynamic.rs (continued)
pub struct BranchCompleter {
    git_service: GitService,
}

impl BranchCompleter {
    pub fn complete_branch_names(&self, prefix: &str) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Local branches
        for branch in self.git_service.list_local_branches()? {
            if branch.starts_with(prefix) {
                let is_current = self.git_service.get_current_branch()? == branch;
                suggestions.push(CompletionSuggestion {
                    value: branch.clone(),
                    description: Some(if is_current { 
                        "Current branch".to_string() 
                    } else { 
                        "Local branch".to_string() 
                    }),
                    completion_type: CompletionType::BranchName,
                    priority: if is_current { 10 } else { 8 },
                });
            }
        }
        
        // Remote branches (lower priority)
        for branch in self.git_service.list_remote_branches()? {
            if branch.starts_with(prefix) {
                suggestions.push(CompletionSuggestion {
                    value: branch.strip_prefix("origin/").unwrap_or(&branch).to_string(),
                    description: Some("Remote branch".to_string()),
                    completion_type: CompletionType::BranchName,
                    priority: 6,
                });
            }
        }
        
        // Common branch name patterns
        if prefix.is_empty() || "feature/".starts_with(prefix) {
            suggestions.push(CompletionSuggestion {
                value: "feature/".to_string(),
                description: Some("Feature branch prefix".to_string()),
                completion_type: CompletionType::BranchName,
                priority: 7,
            });
        }
        
        Ok(suggestions)
    }
}
```

### 4. File Path Completion
Enhanced file completion for `--file` flags:

```rust
// src/cli/completion/dynamic.rs (continued)
pub struct FileCompleter {
    current_dir: PathBuf,
}

impl FileCompleter {
    pub fn complete_file_paths(&self, prefix: &str, command: &str) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        let path_prefix = Path::new(prefix);
        
        let search_dir = if path_prefix.is_absolute() {
            path_prefix.parent().unwrap_or(Path::new("/")).to_path_buf()
        } else if prefix.contains('/') {
            self.current_dir.join(path_prefix.parent().unwrap_or(Path::new(".")))
        } else {
            self.current_dir.clone()
        };
        
        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy();
                
                if name.starts_with(prefix) || prefix.contains('/') {
                    let is_file = path.is_file();
                    let is_dir = path.is_dir();
                    
                    // For dispatch --file, prioritize text files
                    if command == "dispatch" && is_file {
                        let priority = match path.extension().and_then(|e| e.to_str()) {
                            Some("md") => 10,
                            Some("txt") => 9,
                            Some("prompt") => 10,
                            Some("text") => 8,
                            _ => 6,
                        };
                        
                        suggestions.push(CompletionSuggestion {
                            value: if prefix.contains('/') {
                                path.to_string_lossy().to_string()
                            } else {
                                name.to_string()
                            },
                            description: Some("Text file".to_string()),
                            completion_type: CompletionType::FilePath,
                            priority,
                        });
                    } else if is_dir {
                        suggestions.push(CompletionSuggestion {
                            value: format!("{}/", name),
                            description: Some("Directory".to_string()),
                            completion_type: CompletionType::FilePath,
                            priority: 7,
                        });
                    }
                }
            }
        }
        
        // Add common task files if they exist
        for task_file in &["TASK_*.md", "requirements.txt", "prompt.txt", "task.prompt"] {
            if let Ok(matches) = glob::glob(&format!("{}/{}", search_dir.display(), task_file)) {
                for path in matches.flatten() {
                    if path.file_name().unwrap().to_string_lossy().starts_with(prefix) {
                        suggestions.push(CompletionSuggestion {
                            value: path.file_name().unwrap().to_string_lossy().to_string(),
                            description: Some("Task file".to_string()),
                            completion_type: CompletionType::FilePath,
                            priority: 12,
                        });
                    }
                }
            }
        }
        
        Ok(suggestions)
    }
}
```

### 5. Enhanced Command-Specific Completion
Implement completion for specific command contexts:

```rust
// src/cli/completion/generators.rs
pub struct EnhancedCompletionGenerator {
    session_completer: SessionCompleter,
    branch_completer: BranchCompleter,
    file_completer: FileCompleter,
    config: Config,
}

impl EnhancedCompletionGenerator {
    pub fn generate_completions(&self, context: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        match context.command.as_str() {
            "start" => self.complete_start_command(context),
            "dispatch" => self.complete_dispatch_command(context),
            "finish" => self.complete_finish_command(context),
            "integrate" => self.complete_integrate_command(context),
            "cancel" | "resume" => self.complete_session_command(context),
            "recover" => self.complete_recover_command(context),
            "config" => self.complete_config_command(context),
            _ => Ok(vec![]),
        }
    }
    
    fn complete_dispatch_command(&self, context: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        
        // Check if we're completing a flag
        if context.current_arg.starts_with('-') {
            suggestions.extend(vec![
                CompletionSuggestion {
                    value: "--file".to_string(),
                    description: Some("Read prompt from file".to_string()),
                    completion_type: CompletionType::Flag,
                    priority: 10,
                },
                CompletionSuggestion {
                    value: "--dangerously-skip-permissions".to_string(),
                    description: Some("Skip IDE permission prompts".to_string()),
                    completion_type: CompletionType::Flag,
                    priority: 8,
                },
            ]);
        }
        // If previous arg was --file, complete file paths
        else if context.previous_args.last() == Some(&"--file".to_string()) || 
                context.previous_args.last() == Some(&"-f".to_string()) {
            suggestions.extend(self.file_completer.complete_file_paths(&context.current_arg, "dispatch")?);
        }
        // Complete session names or file paths (auto-detect)
        else {
            // Try file completion first
            if context.current_arg.contains('/') || 
               context.current_arg.ends_with(".md") || 
               context.current_arg.ends_with(".txt") {
                suggestions.extend(self.file_completer.complete_file_paths(&context.current_arg, "dispatch")?);
            }
            
            // Add session name suggestions if no file matches
            if suggestions.is_empty() {
                suggestions.extend(self.session_completer.complete_session_names(&context.current_arg)?);
            }
        }
        
        Ok(suggestions)
    }
    
    fn complete_finish_command(&self, context: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        
        if context.current_arg.starts_with('-') {
            suggestions.extend(vec![
                CompletionSuggestion {
                    value: "--branch".to_string(),
                    description: Some("Rename branch after finish".to_string()),
                    completion_type: CompletionType::Flag,
                    priority: 10,
                },
                CompletionSuggestion {
                    value: "--integrate".to_string(),
                    description: Some("Integrate into base branch".to_string()),
                    completion_type: CompletionType::Flag,
                    priority: 9,
                },
            ]);
        } else if context.previous_args.last() == Some(&"--branch".to_string()) {
            suggestions.extend(self.branch_completer.complete_branch_names(&context.current_arg)?);
        } else {
            // Complete session names
            suggestions.extend(self.session_completer.complete_for_command("finish", &context.current_arg)?);
        }
        
        Ok(suggestions)
    }
    
    fn complete_integrate_command(&self, context: &CompletionContext) -> Result<Vec<CompletionSuggestion>> {
        let mut suggestions = Vec::new();
        
        if context.current_arg.starts_with('-') {
            suggestions.extend(vec![
                CompletionSuggestion {
                    value: "--strategy".to_string(),
                    description: Some("Integration strategy".to_string()),
                    completion_type: CompletionType::Flag,
                    priority: 10,
                },
            ]);
        } else if context.previous_args.last() == Some(&"--strategy".to_string()) {
            suggestions.extend(vec![
                CompletionSuggestion {
                    value: "merge".to_string(),
                    description: Some("Merge commits preserving history".to_string()),
                    completion_type: CompletionType::IntegrationStrategy,
                    priority: 10,
                },
                CompletionSuggestion {
                    value: "squash".to_string(),
                    description: Some("Squash all commits into one".to_string()),
                    completion_type: CompletionType::IntegrationStrategy,
                    priority: 9,
                },
                CompletionSuggestion {
                    value: "rebase".to_string(),
                    description: Some("Rebase onto base branch".to_string()),
                    completion_type: CompletionType::IntegrationStrategy,
                    priority: 8,
                },
            ]);
        } else {
            suggestions.extend(self.session_completer.complete_for_command("integrate", &context.current_arg)?);
        }
        
        Ok(suggestions)
    }
}
```

### 6. Caching and Performance
Implement completion caching for performance:

```rust
// src/cli/completion/cache.rs
use std::time::{Duration, Instant};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CachedCompletion {
    pub suggestions: Vec<CompletionSuggestion>,
    pub timestamp: Instant,
    pub ttl: Duration,
}

pub struct CompletionCache {
    cache: HashMap<String, CachedCompletion>,
    default_ttl: Duration,
}

impl CompletionCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl: Duration::from_secs(30), // 30 second default TTL
        }
    }
    
    pub fn get(&mut self, key: &str) -> Option<Vec<CompletionSuggestion>> {
        if let Some(cached) = self.cache.get(key) {
            if cached.timestamp.elapsed() < cached.ttl {
                return Some(cached.suggestions.clone());
            } else {
                self.cache.remove(key);
            }
        }
        None
    }
    
    pub fn insert(&mut self, key: String, suggestions: Vec<CompletionSuggestion>, ttl: Option<Duration>) {
        let cached = CachedCompletion {
            suggestions,
            timestamp: Instant::now(),
            ttl: ttl.unwrap_or(self.default_ttl),
        };
        self.cache.insert(key, cached);
    }
    
    pub fn invalidate_sessions(&mut self) {
        self.cache.retain(|key, _| !key.starts_with("sessions:"));
    }
}
```

### 7. Shell Integration
Generate enhanced completion scripts:

```rust
// src/cli/completion/generators.rs (continued)
impl EnhancedCompletionGenerator {
    pub fn generate_bash_completion(&self) -> String {
        r#"
_para_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Get dynamic completions from para itself
    local completions
    completions=$(para _completion_dynamic "${COMP_WORDS[@]}" 2>/dev/null)
    
    if [[ ${cur} == -* ]]; then
        # Complete flags
        COMPREPLY=( $(compgen -W "${completions}" -- ${cur}) )
    else
        # Complete values
        COMPREPLY=( $(compgen -W "${completions}" -- ${cur}) )
    fi
}

complete -F _para_completion para
"#.to_string()
    }
    
    pub fn generate_zsh_completion(&self) -> String {
        r#"
#compdef para

_para() {
    local state line
    
    _arguments -C \
        '1: :_para_commands' \
        '*::arg:->args'
    
    case $state in
        args)
            case ${words[2]} in
                start)
                    _arguments \
                        '--dangerously-skip-permissions[Skip IDE permissions]' \
                        '1:session-name:_para_sessions'
                    ;;
                dispatch)
                    _arguments \
                        '--file[Read from file]:file:_files' \
                        '--dangerously-skip-permissions[Skip IDE permissions]' \
                        '1:session-or-prompt:_para_session_or_file' \
                        '2:prompt:'
                    ;;
                finish|integrate)
                    _arguments \
                        '--branch[Target branch]:branch:_para_branches' \
                        '--integrate[Auto-integrate]' \
                        '1:session:_para_active_sessions' \
                        '2:message:'
                    ;;
                cancel|resume)
                    _arguments '1:session:_para_active_sessions'
                    ;;
                recover)
                    _arguments '1:session:_para_archived_sessions'
                    ;;
            esac
            ;;
    esac
}

_para_sessions() {
    local sessions
    sessions=(${(f)"$(para _completion_sessions 2>/dev/null)"})
    _describe 'session' sessions
}

_para_active_sessions() {
    local sessions
    sessions=(${(f)"$(para _completion_active_sessions 2>/dev/null)"})
    _describe 'active session' sessions
}

_para_archived_sessions() {
    local sessions
    sessions=(${(f)"$(para _completion_archived_sessions 2>/dev/null)"})
    _describe 'archived session' sessions
}

_para_branches() {
    local branches
    branches=(${(f)"$(para _completion_branches 2>/dev/null)"})
    _describe 'branch' branches
}

_para
"#.to_string()
    }
}
```

## Implementation Details

### Files to Create/Modify
1. `para-rs/src/cli/completion/mod.rs` - Main completion module
2. `para-rs/src/cli/completion/context.rs` - Completion context handling
3. `para-rs/src/cli/completion/dynamic.rs` - Dynamic completion logic
4. `para-rs/src/cli/completion/generators.rs` - Shell script generation
5. `para-rs/src/cli/completion/cache.rs` - Completion caching
6. Update `para-rs/src/cli/parser.rs` to include completion commands
7. Update existing commands to support `_completion_*` subcommands

### Dependencies to Add
```toml
[dependencies]
glob = "0.3"  # For file pattern matching
```

### Testing Strategy
1. **Unit tests** for each completion type
2. **Integration tests** with mock git repositories
3. **Performance tests** for large session/branch lists
4. **Shell integration tests** for bash/zsh/fish
5. **Caching tests** for TTL and invalidation

### Performance Considerations
- Cache session lists for 30 seconds
- Use lazy loading for git operations
- Limit completion results to reasonable numbers (50-100)
- Background refresh for frequently used completions

## Legacy Reference
Study completion patterns in:
- `lib/para-commands.sh` completion functions
- Shell completion scripts in legacy implementation
- Session and branch listing logic

## Validation Criteria
- [ ] Session name completion works for all commands
- [ ] Branch completion works for `--branch` flags
- [ ] File path completion works for `--file` flags
- [ ] Integration strategy completion works
- [ ] Completion is fast (< 100ms for normal cases)
- [ ] Caching reduces redundant operations
- [ ] All shell types (bash/zsh/fish) work correctly
- [ ] Error handling is graceful for missing git/sessions
- [ ] Legacy compatibility is maintained

## Completion
When complete, call `para finish "Implement enhanced shell completion with context awareness"` to finish the task.

The agent should focus on getting the core completion engine working first, then add shell-specific optimizations and caching for performance.
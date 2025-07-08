use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Common sandbox arguments shared across multiple commands
#[derive(Args, Debug, Clone)]
pub struct SandboxArgs {
    /// Enable sandboxing (overrides config)
    #[arg(
        long = "sandbox",
        short = 's',
        help = "Enable sandboxing for Claude CLI (overrides config)"
    )]
    pub sandbox: bool,

    /// Disable sandboxing (overrides config)
    #[arg(
        long = "no-sandbox",
        help = "Disable sandboxing for Claude CLI (overrides config)"
    )]
    pub no_sandbox: bool,

    /// Sandbox profile to use
    #[arg(
        long = "sandbox-profile",
        help = "Sandbox profile to use: permissive (default) or restrictive"
    )]
    pub sandbox_profile: Option<String>,

    /// Enable network-isolated sandboxing
    #[arg(
        long = "sandbox-no-network",
        conflicts_with = "sandbox",
        help = "Enable sandboxing with network isolation via proxy"
    )]
    pub sandbox_no_network: bool,

    /// Additional allowed domains for network sandboxing (comma-separated)
    #[arg(
        long = "allowed-domains",
        value_delimiter = ',',
        requires = "sandbox_no_network",
        help = "Additional domains allowed through network proxy (e.g., npmjs.org,pypi.org)"
    )]
    pub allowed_domains: Vec<String>,
}

#[derive(Parser)]
#[command(name = "para")]
#[command(about = "Parallel IDE Workflow Helper")]
#[command(
    version,
    long_about = "When run without any command, opens the monitor view to manage active sessions"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create new para sessions (interactive or AI-assisted)
    Start(UnifiedStartArgs),
    /// Complete session and create feature branch for review
    Finish(FinishArgs),
    /// Cancel session (moves to archive)
    Cancel(CancelArgs),
    /// Remove all active sessions
    Clean(CleanArgs),
    /// List active sessions
    #[command(alias = "ls")]
    List(ListArgs),
    /// Resume session in IDE
    Resume(ResumeArgs),
    /// Recover cancelled session from archive
    Recover(RecoverArgs),
    /// Setup configuration
    Config(ConfigArgs),
    /// Generate shell completion script
    Completion(CompletionArgs),
    /// Initialize shell completions automatically
    Init,
    /// Setup Model Context Protocol (MCP) integration
    Mcp(crate::cli::commands::mcp::McpCommand),
    /// Legacy completion endpoint for sessions (hidden)
    #[command(name = "_completion_sessions", hide = true)]
    CompletionSessions,
    /// Legacy completion endpoint for branches (hidden)
    #[command(name = "_completion_branches", hide = true)]
    CompletionBranches,
    /// Monitor and manage active sessions in real-time (interactive TUI with mouse support)
    Monitor(MonitorArgs),
    /// Update session status (for agents to communicate progress)
    Status(StatusArgs),
    /// Manage Docker container authentication
    Auth(AuthArgs),
    /// Manage para daemon (internal use)
    #[command(hide = true)]
    Daemon(DaemonArgs),
    /// Run network proxy for sandboxing (internal use)
    #[command(hide = true)]
    Proxy(ProxyArgs),
}

/// Internal args struct for delegation to start command (not exposed in CLI)
#[derive(Args, Debug)]
pub struct StartArgs {
    /// Session name (optional, generates friendly name if not provided)
    pub name: Option<String>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,

    /// Run session in Docker container
    #[arg(long, short = 'c', help = "Run session in Docker container")]
    pub container: bool,

    /// Enable network isolation and allow access to specified domains (comma-separated)
    #[arg(
        long,
        help = "Enable network isolation and allow access to specified domains (comma-separated). Use empty string for default domains only."
    )]
    pub allow_domains: Option<String>,

    /// Additional Docker arguments to pass through
    #[arg(
        long = "docker-args",
        allow_hyphen_values = true,
        help = "Additional Docker arguments to pass through"
    )]
    pub docker_args: Vec<String>,

    /// Path to setup script to run after session creation
    #[arg(
        long = "setup-script",
        help = "Path to setup script to run after session creation (before IDE launch)"
    )]
    pub setup_script: Option<PathBuf>,

    /// Custom Docker image to use instead of the default
    #[arg(
        long,
        help = "Custom Docker image to use (e.g., 'ubuntu:22.04', 'mycompany/dev:latest')\n\
                Priority: 1. CLI flag, 2. Config docker.default_image, 3. Default 'para-authenticated:latest'"
    )]
    pub docker_image: Option<String>,

    /// Disable API key forwarding to the container
    #[arg(
        long,
        help = "Disable automatic API key forwarding to Docker containers"
    )]
    pub no_forward_keys: bool,

    /// Sandbox configuration
    #[command(flatten)]
    pub sandbox_args: SandboxArgs,
}

/// Internal args struct for delegation to dispatch command (not exposed in CLI)
#[derive(Args, Debug)]
pub struct DispatchArgs {
    /// Session name or prompt text
    pub name_or_prompt: Option<String>,

    /// Additional prompt text (when first arg is session name)
    pub prompt: Option<String>,

    /// Read prompt from file
    #[arg(long, short = 'f', help = "Read prompt from specified file")]
    pub file: Option<PathBuf>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, short = 'd', help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,

    /// Run session in Docker container
    #[arg(long, short = 'c', help = "Run session in Docker container")]
    pub container: bool,

    /// Enable network isolation and allow access to specified domains (comma-separated)
    #[arg(
        long,
        help = "Enable network isolation and allow access to specified domains (comma-separated). Use empty string for default domains only."
    )]
    pub allow_domains: Option<String>,

    /// Additional Docker arguments to pass through
    #[arg(
        long = "docker-args",
        allow_hyphen_values = true,
        help = "Additional Docker arguments to pass through"
    )]
    pub docker_args: Vec<String>,

    /// Path to setup script to run after session creation
    #[arg(
        long = "setup-script",
        help = "Path to setup script to run after session creation (before IDE launch)"
    )]
    pub setup_script: Option<PathBuf>,

    /// Custom Docker image to use instead of the default
    #[arg(
        long,
        help = "Custom Docker image to use (e.g., 'ubuntu:22.04', 'mycompany/dev:latest')\n\
                Priority: 1. CLI flag, 2. Config docker.default_image, 3. Default 'para-authenticated:latest'"
    )]
    pub docker_image: Option<String>,

    /// Disable API key forwarding to the container
    #[arg(
        long,
        help = "Disable automatic API key forwarding to Docker containers"
    )]
    pub no_forward_keys: bool,

    /// Sandbox configuration
    #[command(flatten)]
    pub sandbox_args: SandboxArgs,
}

#[derive(Args, Debug)]
#[command(after_help = "EXAMPLES:
    # Resume session from current directory (auto-detect)
    para resume
    
    # Resume specific session
    para resume my-feature
    
    # Resume with additional instructions
    para resume my-feature --prompt \"add error handling\"
    
    # Resume with instructions from file
    para resume my-feature --file new-requirements.txt")]
pub struct ResumeArgs {
    /// Session ID to resume (optional, auto-detects from current directory if not provided)
    pub session: Option<String>,

    /// Additional prompt or instructions for the resumed session
    #[arg(long, short)]
    pub prompt: Option<String>,

    /// Read additional instructions from specified file
    #[arg(long, short)]
    pub file: Option<PathBuf>,

    /// Skip IDE permission warnings (DANGEROUS: Only use for automated scripts)
    #[arg(
        long,
        help = "Skip IDE permission warnings (DANGEROUS: Only use for automated scripts)"
    )]
    pub dangerously_skip_permissions: bool,

    /// Sandbox configuration
    #[command(flatten)]
    pub sandbox_args: SandboxArgs,
}

#[derive(Args, Debug)]
pub struct FinishArgs {
    /// Commit message
    pub message: String,

    /// Custom branch name after finishing
    #[arg(long, short = 'b', help = "Rename feature branch to specified name")]
    pub branch: Option<String>,

    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,

    /// Force cancellation even with uncommitted changes (destructive)
    #[arg(
        long,
        short,
        help = "Force cancellation even with uncommitted changes (destructive)"
    )]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Skip confirmation prompts
    #[arg(long, short, help = "Skip confirmation prompts")]
    pub force: bool,

    /// Only show what would be cleaned (dry run)
    #[arg(long, help = "Only show what would be cleaned (dry run)")]
    pub dry_run: bool,

    /// Also clean backup archives (deprecated, use default behavior)
    #[arg(long, help = "Also remove archived sessions", hide = true)]
    pub backups: bool,

    /// Clean orphaned Docker containers
    #[arg(long, help = "Clean orphaned Docker containers")]
    pub containers: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show additional session details
    #[arg(long, short = 'v', help = "Show verbose session information")]
    pub verbose: bool,

    /// Show archived sessions
    #[arg(long, short = 'a', help = "Show archived sessions")]
    pub archived: bool,

    /// Quiet output (minimal formatting for completion)
    #[arg(long, short = 'q', help = "Quiet output for completion")]
    pub quiet: bool,
}

#[derive(Args, Debug)]
pub struct RecoverArgs {
    /// Session ID to recover from archive (optional, shows list if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommands>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Interactive configuration wizard
    Setup,
    /// Auto-detect and configure IDE
    Auto,
    /// Show current configuration
    Show,
    /// Edit configuration file
    Edit,
    /// Reset configuration to defaults
    Reset,
    /// Set configuration value using JSON path
    Set {
        /// JSON path using dot notation (e.g., ide.name, git.auto_stage)
        path: String,
        /// Value to set
        value: String,
    },
    /// Manage project-level configuration
    Project {
        #[command(subcommand)]
        command: Option<ProjectConfigCommands>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ProjectConfigCommands {
    /// Initialize project configuration
    Init,
    /// Show project configuration
    Show,
    /// Edit project configuration
    Edit,
    /// Set project configuration value
    Set {
        /// JSON path using dot notation (e.g., sandbox.enabled, ide.preferred)
        path: String,
        /// Value to set
        value: String,
    },
}

#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completion for, or 'init' for automatic setup
    pub shell: String,
}

#[derive(Args, Debug)]
pub struct MonitorArgs {}

#[derive(Args, Debug)]
pub struct StatusArgs {
    #[command(subcommand)]
    pub command: Option<StatusCommands>,

    /// Current task description (for backwards compatibility)
    pub task: Option<String>,

    /// Test status (passed, failed, unknown)
    #[arg(long, help = "Test status: passed, failed, or unknown")]
    pub tests: Option<String>,

    /// Todo progress (format: completed/total, e.g., 3/7)
    #[arg(long, help = "Todo progress in format 'completed/total' (e.g., '3/7')")]
    pub todos: Option<String>,

    /// Mark as blocked
    #[arg(long, help = "Mark session as blocked")]
    pub blocked: bool,

    /// Session name (optional, auto-detects from current directory)
    #[arg(long, help = "Session name (auto-detected if not provided)")]
    pub session: Option<String>,
}

/// Start command arguments (creates new sessions, interactive or AI-assisted)
#[derive(Args, Debug)]
#[command(after_help = "EXAMPLES:
    # Start new interactive session
    para start
    para start feature-xyz
    
    # Start new session with AI agent (dispatch functionality)
    para start \"implement user authentication\"
    para start feature-xyz \"implement authentication\"
    
    # Use task file
    para start --file tasks/auth.md
    para start feature-xyz --file context.md
    
    # Docker container sessions
    para start --container \"implement feature\"
    para start --container --allow-domains github.com,api.example.com")]
pub struct UnifiedStartArgs {
    /// Session name, existing session, or prompt text
    pub name_or_session: Option<String>,

    /// Additional prompt text (when first arg is session name)
    pub prompt: Option<String>,

    /// Read prompt/context from file
    #[arg(long, short = 'f', help = "Read prompt or context from specified file")]
    pub file: Option<PathBuf>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, short = 'd', help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,

    /// Run session in Docker container
    #[arg(long, short = 'c', help = "Run session in Docker container")]
    pub container: bool,

    /// Enable network isolation and allow access to specified domains
    #[arg(
        long,
        help = "Enable network isolation and allow access to specified domains (comma-separated)"
    )]
    pub allow_domains: Option<String>,

    /// Additional Docker arguments to pass through
    #[arg(
        long = "docker-args",
        allow_hyphen_values = true,
        help = "Additional Docker arguments to pass through"
    )]
    pub docker_args: Vec<String>,

    /// Path to setup script to run after session creation
    #[arg(
        long = "setup-script",
        help = "Path to setup script to run after session creation"
    )]
    pub setup_script: Option<PathBuf>,

    /// Custom Docker image to use
    #[arg(long, help = "Custom Docker image to use (e.g., 'ubuntu:22.04')")]
    pub docker_image: Option<String>,

    /// Disable API key forwarding to container
    #[arg(
        long,
        help = "Disable automatic API key forwarding to Docker containers"
    )]
    pub no_forward_keys: bool,

    /// Sandbox configuration
    #[command(flatten)]
    pub sandbox_args: SandboxArgs,
}

#[derive(Subcommand, Debug)]
pub enum StatusCommands {
    /// Show status of one or all sessions
    Show {
        /// Session name (optional, shows all if not provided)
        session: Option<String>,

        /// Output format
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
    /// Generate a summary of all status files
    Summary {
        /// Output format
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
    /// Clean up stale status files
    Cleanup {
        /// Dry run - show what would be cleaned without removing
        #[arg(long, help = "Show what would be cleaned without removing")]
        dry_run: bool,
    },
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: Option<AuthCommands>,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Set up container authentication interactively
    Setup {
        /// Force re-authentication even if credentials exist
        #[arg(long, help = "Force re-authentication even if credentials exist")]
        force: bool,
    },
    /// Remove authentication artifacts
    Cleanup {
        /// Show what would be removed without actually removing
        #[arg(long, help = "Show what would be removed without actually removing")]
        dry_run: bool,
    },
    /// Check authentication status
    Status {
        /// Show detailed authentication information
        #[arg(long, help = "Show detailed authentication information")]
        verbose: bool,
    },
    /// Re-authenticate (cleanup and setup in one command)
    Reauth,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl FinishArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if self.message.trim().is_empty() {
            return Err(crate::utils::ParaError::invalid_args(
                "Commit message cannot be empty",
            ));
        }

        if let Some(ref branch) = self.branch {
            validate_branch_name(branch)?;
        }

        Ok(())
    }
}

pub fn validate_session_name(name: &str) -> crate::utils::Result<()> {
    if name.is_empty() {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name cannot be empty",
        ));
    }

    if name.len() > 50 {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name too long (max 50 characters)",
        ));
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(crate::utils::ParaError::invalid_args(
            "Session name can only contain alphanumeric characters, hyphens, and underscores",
        ));
    }

    Ok(())
}

pub fn validate_branch_name(name: &str) -> crate::utils::Result<()> {
    if name.is_empty() {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name cannot be empty",
        ));
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name cannot start or end with hyphen",
        ));
    }

    if name.contains("..") || name.contains("//") {
        return Err(crate::utils::ParaError::invalid_args(
            "Branch name contains invalid character sequence",
        ));
    }

    Ok(())
}

#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommands,
}

/// Proxy arguments for network sandboxing
#[derive(Args, Debug)]
pub struct ProxyArgs {
    /// Port to run the proxy on
    #[arg(long, default_value = "8877")]
    pub port: u16,

    /// Additional domains to allow (comma-separated)
    #[arg(long)]
    pub allowed_domains: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum DaemonCommands {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
}

impl UnifiedStartArgs {
    /// Validate the unified start arguments
    pub fn validate(&self) -> crate::utils::Result<()> {
        // Can't specify both --prompt and --file
        if self.prompt.is_some() && self.file.is_some() {
            return Err(crate::utils::ParaError::invalid_args(
                "Cannot specify both --prompt and --file",
            ));
        }

        // Validate sandbox args
        if self.sandbox_args.sandbox && self.sandbox_args.no_sandbox {
            return Err(crate::utils::ParaError::invalid_args(
                "Cannot specify both --sandbox and --no-sandbox",
            ));
        }

        Ok(())
    }

    /// Convert to StartArgs for delegating to existing start command
    pub fn to_start_args(&self, name: Option<String>) -> StartArgs {
        StartArgs {
            name,
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            container: self.container,
            allow_domains: self.allow_domains.clone(),
            docker_args: self.docker_args.clone(),
            setup_script: self.setup_script.clone(),
            docker_image: self.docker_image.clone(),
            no_forward_keys: self.no_forward_keys,
            sandbox_args: self.sandbox_args.clone(),
        }
    }

    /// Convert to DispatchArgs for delegating to existing dispatch command
    pub fn to_dispatch_args(&self, name: Option<String>, prompt: Option<String>) -> DispatchArgs {
        let has_name = name.is_some();
        DispatchArgs {
            name_or_prompt: name.or(prompt.clone()),
            prompt: if has_name { prompt } else { None },
            file: self.file.clone(),
            dangerously_skip_permissions: self.dangerously_skip_permissions,
            container: self.container,
            allow_domains: self.allow_domains.clone(),
            docker_args: self.docker_args.clone(),
            setup_script: self.setup_script.clone(),
            docker_image: self.docker_image.clone(),
            no_forward_keys: self.no_forward_keys,
            sandbox_args: self.sandbox_args.clone(),
        }
    }
}

// Implementation blocks for internal structs
impl StartArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if let Some(ref name) = self.name {
            validate_session_name(name)?;
        }
        Ok(())
    }
}

impl DispatchArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        use std::io::IsTerminal;

        // Allow no arguments if stdin is piped
        if !std::io::stdin().is_terminal() {
            return Ok(());
        }

        self.validate_args()
    }

    fn validate_args(&self) -> crate::utils::Result<()> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (None, None, None) => Err(crate::utils::ParaError::invalid_args(
                "dispatch requires a prompt text or file path",
            )),
            _ => Ok(()),
        }
    }
}

impl ResumeArgs {
    pub fn validate(&self) -> crate::utils::Result<()> {
        if self.prompt.is_some() && self.file.is_some() {
            return Err(crate::utils::ParaError::invalid_args(
                "Cannot specify both --prompt and --file. Please use only one.",
            ));
        }
        Ok(())
    }
}

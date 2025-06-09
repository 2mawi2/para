use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "para")]
#[command(about = "Parallel IDE Workflow Helper")]
#[command(version, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create session with optional name
    Start(StartArgs),
    /// Start Claude Code session with prompt
    Dispatch(DispatchArgs),
    /// Squash all changes into single commit
    Finish(FinishArgs),
    /// Squash commits and merge into base branch
    Integrate(IntegrateArgs),
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
    /// Complete merge after resolving conflicts
    Continue,
    /// Setup configuration
    Config(ConfigArgs),
    /// Generate shell completion script
    Completion(CompletionArgs),
    /// Dynamic completion (hidden)
    #[command(hide = true)]
    CompleteCommand(CompleteCommandArgs),
}

#[derive(Args, Debug)]
pub struct StartArgs {
    /// Session name (optional, generates friendly name if not provided)
    pub name: Option<String>,

    /// Skip IDE permission warnings (dangerous)
    #[arg(long, help = "Skip IDE permission warnings (dangerous)")]
    pub dangerously_skip_permissions: bool,
}

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
}

#[derive(Args, Debug)]
pub struct FinishArgs {
    /// Commit message
    pub message: String,

    /// Custom branch name after finishing
    #[arg(long, help = "Rename feature branch to specified name")]
    pub branch: Option<String>,

    /// Automatically integrate into base branch
    #[arg(long, short = 'i', help = "Automatically integrate into base branch")]
    pub integrate: bool,

    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct IntegrateArgs {
    /// Commit message for integration
    pub message: Option<String>,

    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,

    /// Integration strategy to use
    #[arg(long, value_enum, help = "Choose integration strategy")]
    pub strategy: Option<IntegrationStrategy>,

    /// Target branch to integrate into
    #[arg(long, help = "Integrate into specific target branch")]
    pub target: Option<String>,

    /// Preview integration without executing
    #[arg(long, help = "Preview integration without executing")]
    pub dry_run: bool,

    /// Abort integration and restore original state
    #[arg(long, help = "Abort integration and restore original state")]
    pub abort: bool,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Session ID (optional, auto-detects if not provided)
    pub session: Option<String>,
}

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Also clean backup archives
    #[arg(long, help = "Also remove archived sessions")]
    pub backups: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show additional session details
    #[arg(long, short = 'v', help = "Show verbose session information")]
    pub verbose: bool,

    /// Show archived sessions
    #[arg(long, short = 'a', help = "Show archived sessions")]
    pub archived: bool,
}

#[derive(Args, Debug)]
pub struct ResumeArgs {
    /// Session ID to resume (optional, shows list if not provided)
    pub session: Option<String>,
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
}

#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completion for
    #[arg(value_enum)]
    pub shell: Shell,
}

#[derive(Args, Debug)]
pub struct CompleteCommandArgs {
    /// Current command line being completed
    #[arg(long)]
    pub command_line: String,

    /// Current word being completed
    #[arg(long)]
    pub current_word: String,

    /// Previous word in command line
    #[arg(long)]
    pub previous_word: Option<String>,

    /// Position of current word
    #[arg(long)]
    pub position: usize,
}

#[derive(ValueEnum, Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

#[derive(ValueEnum, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum IntegrationStrategy {
    /// Create merge commit preserving feature branch history
    Merge,
    /// Combine all feature branch commits into single commit
    Squash,
    /// Replay feature branch commits on target branch
    Rebase,
}

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
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (None, None, None) => Err(crate::utils::ParaError::invalid_args(
                "dispatch requires a prompt text or file path",
            )),
            _ => Ok(()),
        }
    }
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

impl Commands {
    #[allow(dead_code)]
    pub fn examples(&self) -> &'static str {
        match self {
            Commands::Start(_) => {
                "Examples:\n  para start\n  para start my-feature\n  para start --dangerously-skip-permissions feature-auth"
            }
            Commands::Dispatch(_) => {
                "Examples:\n  para dispatch \"Add user authentication\"\n  para dispatch --file prompt.txt\n  para dispatch auth-feature --file requirements.md"
            }
            Commands::Finish(_) => {
                "Examples:\n  para finish \"Implement user auth\"\n  para finish \"Add login form\" --branch feature-login\n  para finish \"Complete auth\" --integrate"
            }
            Commands::Integrate(_) => {
                "Examples:\n  para integrate\n  para integrate session-123\n  para integrate --strategy merge\n  para integrate --target main --dry-run"
            }
            Commands::Cancel(_) => {
                "Examples:\n  para cancel\n  para cancel session-123"
            }
            Commands::Clean(_) => {
                "Examples:\n  para clean\n  para clean --backups"
            }
            Commands::List(_) => {
                "Examples:\n  para list\n  para ls -v\n  para list --archived"
            }
            Commands::Resume(_) => {
                "Examples:\n  para resume\n  para resume session-123"
            }
            Commands::Recover(_) => {
                "Examples:\n  para recover\n  para recover old-session"
            }
            Commands::Continue => {
                "Examples:\n  para continue"
            }
            Commands::Config(_) => {
                "Examples:\n  para config\n  para config setup\n  para config auto\n  para config show\n  para config edit\n  para config reset"
            }
            Commands::Completion(_) => {
                "Examples:\n  para completion bash\n  para completion zsh > ~/.zsh_completions/_para"
            }
            Commands::CompleteCommand(_) => {
                "Examples:\n  para complete-command --command-line 'para start' --current-word '' --position 2"
            }
        }
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

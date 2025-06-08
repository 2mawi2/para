use clap::{Parser, Subcommand};

mod utils;

#[derive(Parser)]
#[command(name = "para")]
#[command(about = "Parallel IDE Workflow Helper", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create session with optional name
    Start { name: Option<String> },
    /// Start Claude Code session with prompt
    Dispatch { 
        name_or_prompt: Option<String>,
        prompt: Option<String>,
        #[arg(long, short)]
        file: Option<String>,
    },
    /// Squash all changes into single commit
    Finish { 
        message: String,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long, short)]
        integrate: bool,
    },
    /// Squash commits and merge into base branch
    Integrate { message: String },
    /// Cancel session (moves to archive)
    Cancel { session: Option<String> },
    /// Remove all active sessions
    Clean {
        #[arg(long)]
        backups: bool,
    },
    /// List active sessions
    #[command(alias = "ls")]
    List,
    /// Resume session in IDE
    Resume { session: Option<String> },
    /// Recover cancelled session from archive
    Recover { session: Option<String> },
    /// Complete merge after resolving conflicts
    Continue,
    /// Setup configuration
    Config { subcommand: Option<String> },
    /// Generate shell completion script
    Completion { 
        #[command(subcommand)]
        command: Option<CompletionCommands>,
    },
}

#[derive(Subcommand)]
enum CompletionCommands {
    Generate { shell: String },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(command) => handle_command(command),
        None => {
            println!("para - Parallel IDE Workflow Helper");
            println!();
            println!("Commands:");
            println!("para start [name]                    # create session with optional name");
            println!("para dispatch \"prompt\"               # start Claude Code session with prompt");
            println!("para dispatch --file path            # start Claude Code session with prompt from file");
            println!("para finish \"message\"                # squash all changes into single commit");
            println!("para finish \"message\" --branch <n>   # squash commits + custom branch name");
            println!("para finish \"message\" --integrate    # squash commits + merge into base branch");
            println!("para list | ls                       # list active sessions");
            println!("para resume [session]                # resume session in IDE");
            println!("para recover [session]               # recover cancelled session from archive");
            println!("para cancel [session]                # cancel session (moves to archive)");
            println!("para clean                           # remove all active sessions");
            println!("para clean --backups                 # remove all cancelled sessions from archive");
            println!("para continue                        # complete merge after resolving conflicts");
            println!("para config                          # setup configuration");
            println!("para completion generate [shell]     # generate shell completion script");
            println!();
            println!("For configuration help: para config");
        }
    }
}

fn handle_command(command: Commands) {
    match command {
        Commands::Start { name } => {
            eprintln!("para: start command not implemented yet");
            std::process::exit(1);
        }
        Commands::Dispatch { name_or_prompt, prompt, file } => {
            eprintln!("para: dispatch command not implemented yet");
            std::process::exit(1);
        }
        Commands::Finish { message, branch, integrate } => {
            eprintln!("para: finish command not implemented yet");
            std::process::exit(1);
        }
        Commands::Integrate { message } => {
            eprintln!("para: integrate command not implemented yet");
            std::process::exit(1);
        }
        Commands::Cancel { session } => {
            eprintln!("para: cancel command not implemented yet");
            std::process::exit(1);
        }
        Commands::Clean { backups } => {
            eprintln!("para: clean command not implemented yet");
            std::process::exit(1);
        }
        Commands::List => {
            eprintln!("para: list command not implemented yet");
            std::process::exit(1);
        }
        Commands::Resume { session } => {
            eprintln!("para: resume command not implemented yet");
            std::process::exit(1);
        }
        Commands::Recover { session } => {
            eprintln!("para: recover command not implemented yet");
            std::process::exit(1);
        }
        Commands::Continue => {
            eprintln!("para: continue command not implemented yet");
            std::process::exit(1);
        }
        Commands::Config { subcommand } => {
            eprintln!("para: config command not implemented yet");
            std::process::exit(1);
        }
        Commands::Completion { command } => {
            eprintln!("para: completion command not implemented yet");
            std::process::exit(1);
        }
    }
}

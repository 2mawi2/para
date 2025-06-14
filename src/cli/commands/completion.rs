use crate::cli::completion::generators::ShellCompletionGenerator;
use crate::cli::parser::{CompletionArgs, Shell};
use crate::utils::Result;

pub fn execute(args: CompletionArgs) -> Result<()> {
    // Handle special case: user typed "para completion init"
    if args.shell == "init" {
        println!("It looks like you want to set up completions automatically!");
        println!();
        println!("Run this command instead:");
        println!("   para init");
        println!();
        println!("This will automatically detect your shell and install completions.");
        return Ok(());
    }

    // Parse the shell string into Shell enum
    let shell = match args.shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        _ => {
            eprintln!("Error: '{}' is not a supported shell", args.shell);
            eprintln!("Supported shells: bash, zsh, fish");
            eprintln!("For automatic setup, use: para init");
            return Ok(());
        }
    };

    // Check if user wants the raw completion script
    if std::env::var("PARA_COMPLETION_SCRIPT").is_ok() {
        // Just output the raw script for piping/redirecting
        let completion_script = ShellCompletionGenerator::generate_enhanced_completion(shell)?;
        println!("{}", completion_script);
        return Ok(());
    }

    // Check if user wants detailed installation instructions
    if std::env::var("PARA_COMPLETION_HELP").is_ok() {
        println!(
            "{}",
            ShellCompletionGenerator::get_installation_instructions(shell)
        );
        return Ok(());
    }

    // Default user-friendly behavior
    println!("Para shell completions for {:?}", shell);
    println!();
    println!("For automatic setup, run:");
    println!("   para init");
    println!();
    println!("For manual installation:");
    match shell {
        Shell::Bash => {
            println!(
                "   echo 'eval \"$(PARA_COMPLETION_SCRIPT=1 para completion bash)\"' >> ~/.bashrc"
            );
            println!("   source ~/.bashrc");
        }
        Shell::Zsh => {
            println!(
                "   echo 'eval \"$(PARA_COMPLETION_SCRIPT=1 para completion zsh)\"' >> ~/.zshrc"
            );
            println!("   source ~/.zshrc");
        }
        Shell::Fish => {
            println!("   PARA_COMPLETION_SCRIPT=1 para completion fish > ~/.config/fish/completions/para.fish");
            println!("   # Restart your shell or run: source ~/.config/fish/config.fish");
        }
    }
    println!();
    println!("For detailed options, run:");
    println!("   PARA_COMPLETION_HELP=1 para completion {:?}", shell);

    Ok(())
}

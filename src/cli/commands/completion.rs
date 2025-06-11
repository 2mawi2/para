use crate::cli::completion::generators::ShellCompletionGenerator;
use crate::cli::parser::CompletionArgs;
use crate::utils::Result;

pub fn execute(args: CompletionArgs) -> Result<()> {
    let completion_script =
        ShellCompletionGenerator::generate_enhanced_completion(args.shell.clone())?;
    println!("{}", completion_script);

    if std::env::var("PARA_COMPLETION_HELP").is_ok() {
        eprintln!(
            "\n{}",
            ShellCompletionGenerator::get_installation_instructions(args.shell)
        );
    }

    Ok(())
}

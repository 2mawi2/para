use crate::cli::parser::{CompletionArgs, Shell};
use crate::utils::{ParaError, Result};

pub fn execute(args: CompletionArgs) -> Result<()> {
    let completion_script = generate_completion(args.shell)?;
    println!("{}", completion_script);
    Ok(())
}

fn generate_completion(shell: Shell) -> Result<String> {
    use clap::CommandFactory;
    use clap_complete::{generate, shells};

    let mut cmd = crate::cli::parser::Cli::command();
    let mut buf = Vec::new();

    match shell {
        Shell::Bash => generate(shells::Bash, &mut cmd, "para", &mut buf),
        Shell::Zsh => generate(shells::Zsh, &mut cmd, "para", &mut buf),
        Shell::Fish => generate(shells::Fish, &mut cmd, "para", &mut buf),
        Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "para", &mut buf),
    }

    String::from_utf8(buf)
        .map_err(|e| ParaError::invalid_args(format!("UTF-8 error generating completion: {}", e)))
}

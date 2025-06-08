use clap_complete::{generate, Shell};
use clap_complete::shells::{Bash, Zsh, Fish, PowerShell};
use clap::CommandFactory;
use crate::utils::{Result, ParaError};

#[allow(dead_code)]
pub fn generate_completion(shell: Shell) -> Result<String> {
    let mut cmd = crate::cli::parser::Cli::command();
    let mut buf = Vec::new();
    
    match shell {
        Shell::Bash => generate(Bash, &mut cmd, "para", &mut buf),
        Shell::Zsh => generate(Zsh, &mut cmd, "para", &mut buf),
        Shell::Fish => generate(Fish, &mut cmd, "para", &mut buf),
        Shell::PowerShell => generate(PowerShell, &mut cmd, "para", &mut buf),
        _ => return Err(ParaError::invalid_args(format!("Unsupported shell: {:?}", shell))),
    }
    
    String::from_utf8(buf)
        .map_err(|e| ParaError::invalid_args(format!("UTF-8 error: {}", e)))
}

#[allow(dead_code)]
pub fn generate_enhanced_completion(shell: Shell) -> Result<String> {
    generate_completion(shell)
}
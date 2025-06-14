use chrono::Local;
use dialoguer::Select;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::parser::Shell;
use crate::utils::{ParaError, Result};

fn is_non_interactive() -> bool {
    std::env::var("PARA_NON_INTERACTIVE").is_ok()
        || std::env::var("CI").is_ok()
        || !atty::is(atty::Stream::Stdin)
}

pub fn execute() -> Result<()> {
    println!("Initializing para shell completions...\n");

    let shell = detect_shell()?;
    let config_path = get_shell_config_path(&shell)?;

    println!("Detected shell: {:?}", shell);
    println!("Config file: {}", config_path.display());

    if is_completion_installed(&config_path)? {
        println!("\n✓ Para completions are already installed!");
        println!("  If completions aren't working, try reloading your shell.");
        return Ok(());
    }

    let backup_path = create_backup(&config_path)?;
    println!("Created backup: {}", backup_path.display());

    install_completion(&config_path, &shell)?;

    println!("\n✓ Completions installed successfully!");
    println!("\nTo activate completions, run:");
    match shell {
        Shell::Bash => println!("  source ~/.bashrc"),
        Shell::Zsh => println!("  source ~/.zshrc"),
        Shell::Fish => println!("  source ~/.config/fish/config.fish"),
    }
    println!("\nOr restart your terminal.");

    Ok(())
}

fn detect_shell() -> Result<Shell> {
    if let Ok(shell_env) = env::var("SHELL") {
        if let Some(shell) = parse_shell_from_path(&shell_env) {
            return Ok(shell);
        }
    }

    if let Ok(shell_env) = env::var("0") {
        if let Some(shell) = parse_shell_from_path(&shell_env) {
            return Ok(shell);
        }
    }

    if is_non_interactive() {
        return Err(ParaError::config_error(
            "Cannot auto-detect shell in non-interactive mode. Shell completions require interactive setup."
        ));
    }

    println!("Unable to detect shell automatically.");
    let shells = vec!["bash", "zsh", "fish"];
    let selection = Select::new()
        .with_prompt("Please select your shell")
        .items(&shells)
        .interact()
        .map_err(|e| ParaError::config_error(format!("Failed to get shell selection: {}", e)))?;

    match selection {
        0 => Ok(Shell::Bash),
        1 => Ok(Shell::Zsh),
        2 => Ok(Shell::Fish),
        _ => unreachable!(),
    }
}

fn parse_shell_from_path(path: &str) -> Option<Shell> {
    let shell_name = Path::new(path).file_name()?.to_str()?;
    match shell_name {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        _ => None,
    }
}

fn get_shell_config_path(shell: &Shell) -> Result<PathBuf> {
    let home = env::var("HOME")
        .map_err(|_| ParaError::config_error("HOME environment variable not set"))?;
    let home_path = Path::new(&home);

    let config_path = match shell {
        Shell::Bash => home_path.join(".bashrc"),
        Shell::Zsh => home_path.join(".zshrc"),
        Shell::Fish => home_path.join(".config/fish/config.fish"),
    };

    if shell == &Shell::Fish {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ParaError::fs_error(format!("Failed to create fish config directory: {}", e))
            })?;
        }
    }

    Ok(config_path)
}

fn is_completion_installed(config_path: &Path) -> Result<bool> {
    if !config_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| ParaError::fs_error(format!("Failed to read shell config file: {}", e)))?;

    Ok(content.contains(">>> para completion initialize >>>"))
}

fn create_backup(config_path: &Path) -> Result<PathBuf> {
    if !config_path.exists() {
        return Ok(config_path.with_extension("para-backup-new"));
    }

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_name = format!(
        "{}.para-backup-{}",
        config_path.file_name().unwrap().to_str().unwrap(),
        timestamp
    );

    let backup_path = config_path.parent().unwrap().join(backup_name);

    fs::copy(config_path, &backup_path)
        .map_err(|e| ParaError::fs_error(format!("Failed to create backup: {}", e)))?;

    Ok(backup_path)
}

fn install_completion(config_path: &Path, shell: &Shell) -> Result<()> {
    let completion_block = format!(
        "\n# >>> para completion initialize >>>\neval \"$(PARA_COMPLETION_SCRIPT=1 para completion {})\"\n# <<< para completion initialize <<<\n",
        match shell {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    );

    if config_path.exists() {
        let mut content = fs::read_to_string(config_path)
            .map_err(|e| ParaError::fs_error(format!("Failed to read shell config: {}", e)))?;

        content.push_str(&completion_block);

        fs::write(config_path, content)
            .map_err(|e| ParaError::fs_error(format!("Failed to write shell config: {}", e)))?;
    } else {
        fs::write(config_path, &completion_block)
            .map_err(|e| ParaError::fs_error(format!("Failed to create shell config: {}", e)))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_shell_from_path() {
        assert_eq!(parse_shell_from_path("/bin/bash"), Some(Shell::Bash));
        assert_eq!(parse_shell_from_path("/usr/bin/zsh"), Some(Shell::Zsh));
        assert_eq!(
            parse_shell_from_path("/usr/local/bin/fish"),
            Some(Shell::Fish)
        );
        assert_eq!(parse_shell_from_path("/bin/sh"), None);
        assert_eq!(parse_shell_from_path("bash"), Some(Shell::Bash));
    }

    #[test]
    fn test_is_completion_installed() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".bashrc");

        assert!(!is_completion_installed(&config_path).unwrap());

        fs::write(&config_path, "# Some config\n").unwrap();
        assert!(!is_completion_installed(&config_path).unwrap());

        fs::write(
            &config_path,
            "# >>> para completion initialize >>>\neval \"$(PARA_COMPLETION_SCRIPT=1 para completion bash)\"\n# <<< para completion initialize <<<\n"
        ).unwrap();
        assert!(is_completion_installed(&config_path).unwrap());
    }

    #[test]
    fn test_create_backup() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".bashrc");

        let backup = create_backup(&config_path).unwrap();
        assert!(backup.to_string_lossy().contains("para-backup"));

        fs::write(&config_path, "test content").unwrap();
        let backup2 = create_backup(&config_path).unwrap();
        assert!(backup2.exists());
        assert_eq!(fs::read_to_string(&backup2).unwrap(), "test content");
    }

    #[test]
    fn test_install_completion_new_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".bashrc");

        install_completion(&config_path, &Shell::Bash).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains(">>> para completion initialize >>>"));
        assert!(content.contains("eval \"$(PARA_COMPLETION_SCRIPT=1 para completion bash)\""));
        assert!(content.contains("<<< para completion initialize <<<"));
    }

    #[test]
    fn test_install_completion_existing_file() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".zshrc");

        fs::write(
            &config_path,
            "# Existing config\nexport PATH=$PATH:/usr/local/bin\n",
        )
        .unwrap();

        install_completion(&config_path, &Shell::Zsh).unwrap();

        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.starts_with("# Existing config"));
        assert!(content.contains(">>> para completion initialize >>>"));
        assert!(content.contains("eval \"$(PARA_COMPLETION_SCRIPT=1 para completion zsh)\""));
    }

    #[test]
    fn test_idempotency() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join(".bashrc");

        install_completion(&config_path, &Shell::Bash).unwrap();
        assert!(is_completion_installed(&config_path).unwrap());

        let content_before = fs::read_to_string(&config_path).unwrap();
        let count_before = content_before
            .matches(">>> para completion initialize >>>")
            .count();
        assert_eq!(count_before, 1);
    }

    #[test]
    fn test_fish_config_directory_creation() {
        let temp = TempDir::new().unwrap();
        let home = temp.path();

        env::set_var("HOME", home);

        let config_path = get_shell_config_path(&Shell::Fish).unwrap();
        assert!(config_path.parent().unwrap().exists());

        env::remove_var("HOME");
    }
}

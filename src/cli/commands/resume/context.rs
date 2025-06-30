use crate::cli::parser::ResumeArgs;
use crate::utils::{ParaError, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Process resume context from prompts and files
pub fn process_resume_context(args: &ResumeArgs) -> Result<Option<String>> {
    match (&args.prompt, &args.file) {
        (Some(prompt), None) => Ok(Some(prompt.clone())),
        (None, Some(file_path)) => {
            // Resolve path relative to current directory
            let resolved_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                env::current_dir()?.join(file_path)
            };

            // Validate file exists
            if !resolved_path.exists() {
                return Err(ParaError::fs_error(format!(
                    "File not found: {}",
                    resolved_path.display()
                )));
            }

            // Check file size (1MB limit)
            let metadata = fs::metadata(&resolved_path)?;
            if metadata.len() > 1_048_576 {
                return Err(ParaError::invalid_args(
                    "File too large. Maximum size is 1MB.",
                ));
            }

            // Read file contents
            let content = fs::read_to_string(&resolved_path)
                .map_err(|e| ParaError::fs_error(format!("Failed to read file: {}", e)))?;

            if content.trim().is_empty() {
                println!("⚠️  Warning: File is empty");
            }

            Ok(Some(content))
        }
        (None, None) => Ok(None),
        (Some(_), Some(_)) => unreachable!("Should be caught by validation"),
    }
}

/// Save resume context to a session directory
pub fn save_resume_context(session_path: &Path, session_name: &str, context: &str) -> Result<()> {
    let para_dir = session_path.join(".para");
    let sessions_dir = para_dir.join("sessions");
    let session_dir = sessions_dir.join(session_name);

    // Create directories if they don't exist
    fs::create_dir_all(&session_dir)?;

    // Save context to file
    let context_file = session_dir.join("resume_context.md");
    let mut file = fs::File::create(&context_file)?;
    writeln!(file, "# Resume Context")?;
    writeln!(
        file,
        "This file contains additional context provided when resuming the session.\n"
    )?;
    writeln!(file, "{}", context)?;

    println!("📝 Resume context saved to: {}", context_file.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::{ResumeArgs, SandboxArgs};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_process_resume_context_with_prompt() {
        let args = ResumeArgs {
            session: None,
            prompt: Some("Continue working on the authentication system".to_string()),
            file: None,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = process_resume_context(&args).unwrap();
        assert_eq!(
            result,
            Some("Continue working on the authentication system".to_string())
        );
    }

    #[test]
    fn test_process_resume_context_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("context.md");
        fs::write(&test_file, "# New Requirements\n\nAdd OAuth support").unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(test_file.clone()),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = process_resume_context(&args).unwrap();
        assert_eq!(
            result,
            Some("# New Requirements\n\nAdd OAuth support".to_string())
        );
    }

    #[test]
    fn test_process_resume_context_no_input() {
        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: None,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = process_resume_context(&args).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_process_resume_context_file_not_found() {
        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(PathBuf::from("/nonexistent/file.txt")),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = process_resume_context(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_process_resume_context_file_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("large.txt");

        // Create a file larger than 1MB
        let large_content = "x".repeat(1_048_577);
        fs::write(&test_file, large_content).unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(test_file),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = process_resume_context(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File too large"));
    }

    #[test]
    fn test_save_resume_context() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path();
        let session_name = "test-session";
        let context = "This is test context\nWith multiple lines";

        save_resume_context(session_path, session_name, context).unwrap();

        let expected_file = session_path.join(".para/sessions/test-session/resume_context.md");
        assert!(expected_file.exists());

        let saved_content = fs::read_to_string(&expected_file).unwrap();
        assert!(saved_content.contains("# Resume Context"));
        assert!(saved_content.contains(context));
    }

    #[test]
    fn test_resume_empty_file_warning() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "").unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(empty_file),
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        // Process should succeed but with empty content
        let result = process_resume_context(&args).unwrap();
        assert_eq!(result, Some("".to_string()));
    }
}

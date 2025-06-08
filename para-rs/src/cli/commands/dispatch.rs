use crate::cli::parser::DispatchArgs;
use crate::utils::{Result, ParaError};
use std::path::Path;
use std::fs;

pub fn execute(args: DispatchArgs) -> Result<()> {
    let (session_name, prompt) = args.resolve_prompt_and_session()?;
    
    println!("Dispatch command would execute:");
    println!("  Session: {:?}", session_name);
    println!("  Prompt: {}", prompt);
    println!("  Skip permissions: {}", args.dangerously_skip_permissions);
    
    Err(ParaError::not_implemented("dispatch command"))
}

impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (_, _, Some(file_path)) => {
                let prompt = read_file_content(file_path)?;
                Ok((self.name_or_prompt.clone(), prompt))
            }
            
            (Some(arg), None, None) => {
                if is_likely_file_path(arg) {
                    let prompt = read_file_content(Path::new(arg))?;
                    Ok((None, prompt))
                } else {
                    Ok((None, arg.clone()))
                }
            }
            
            (Some(session), Some(prompt), None) => {
                Ok((Some(session.clone()), prompt.clone()))
            }
            
            (None, None, None) => {
                Err(ParaError::invalid_args("Must provide either a prompt or a file"))
            }
            
            _ => Err(ParaError::invalid_args("Invalid argument combination for dispatch")),
        }
    }
}

fn is_likely_file_path(input: &str) -> bool {
    input.contains('/') || 
    input.ends_with(".txt") || 
    input.ends_with(".md") || 
    input.ends_with(".prompt") ||
    Path::new(input).exists()
}

fn read_file_content(path: &Path) -> Result<String> {
    if !path.exists() {
        return Err(ParaError::invalid_args(format!("File does not exist: {}", path.display())));
    }
    
    fs::read_to_string(path)
        .map_err(|e| ParaError::invalid_args(format!("Failed to read file {}: {}", path.display(), e)))
}
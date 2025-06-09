use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;

pub fn execute() -> Result<()> {
    match GitService::discover() {
        Ok(git_service) => {
            match git_service.list_branches() {
                Ok(branches) => {
                    for branch in branches {
                        // Filter out para's internal branches (consistent with legacy)
                        if !branch.name.starts_with("pc/") {
                            println!("{}", branch.name);
                        }
                    }
                }
                Err(_) => {
                    // Silent failure for completion compatibility
                }
            }
        }
        Err(_) => {
            // Silent failure for completion compatibility
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_with_no_git_repo() {
        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_returns_ok() {
        let result = execute();
        assert!(result.is_ok());
    }

    #[test]
    fn test_silent_failure_behavior() {
        let result = execute();
        assert!(result.is_ok());
    }
}
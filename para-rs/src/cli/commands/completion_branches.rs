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
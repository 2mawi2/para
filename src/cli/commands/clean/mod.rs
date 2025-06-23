use crate::cli::parser::CleanArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::utils::Result;

pub mod coordinator;
pub mod strategies;

use coordinator::CleanupCoordinator;

pub fn execute(config: Config, args: CleanArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let coordinator = CleanupCoordinator::new(git_service, config);
    coordinator.execute_clean(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_args_defaults() {
        let args = CleanArgs {
            force: false,
            dry_run: false,
            backups: false,
        };

        assert!(!args.force);
        assert!(!args.dry_run);
        assert!(!args.backups);
    }
}

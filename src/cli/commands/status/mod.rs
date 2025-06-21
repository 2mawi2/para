mod display;
mod operations;
mod session_detector;

use crate::cli::parser::{StatusArgs, StatusCommands};
use crate::config::Config;
use crate::core::session::SessionManager;
use crate::utils::Result;
use display::{HumanStatusDisplay, JsonStatusDisplay, StatusDisplay, StatusInfo};
use operations::{StatusOperations, StatusUpdate};
use session_detector::SessionDetector;

pub fn execute(config: Config, args: StatusArgs) -> Result<()> {
    let session_manager = SessionManager::new(&config);
    let detector = SessionDetector::new(&session_manager);
    let operations = StatusOperations::new(&session_manager);
    let state_dir = detector.get_state_directory(&config.directories.state_dir)?;

    match args.command {
        Some(StatusCommands::Show { session, json }) => {
            show_status(&operations, &state_dir, session, json)?;
        }
        None => {
            // Backwards compatibility: if no subcommand, this is an update
            if let Some(task) = args.task {
                update_status(&detector, &operations, &state_dir, &args, task)?;
            } else {
                // No task provided and no subcommand, show current status
                let session = detector.find_current_session()?;
                show_status(&operations, &state_dir, Some(session), false)?;
            }
        }
    }

    Ok(())
}

fn show_status(
    operations: &StatusOperations,
    state_dir: &std::path::Path,
    session: Option<String>,
    json: bool,
) -> Result<()> {
    let display: Box<dyn StatusDisplay> = if json {
        Box::new(JsonStatusDisplay)
    } else {
        Box::new(HumanStatusDisplay)
    };

    match session {
        Some(name) => {
            if let Some(status) = operations.get_status(state_dir, &name)? {
                let status_info = StatusInfo::new(status);
                display.display(&status_info)?;
            } else {
                println!("No status found for session '{}'", name);
            }
        }
        None => {
            let statuses = operations.get_all_statuses(state_dir)?;
            let status_infos: Vec<StatusInfo> = statuses
                .into_iter()
                .map(StatusInfo::new)
                .collect();
            display.display_all(&status_infos)?;
        }
    }

    Ok(())
}

fn update_status(
    detector: &SessionDetector,
    operations: &StatusOperations,
    state_dir: &std::path::Path,
    args: &StatusArgs,
    task: String,
) -> Result<()> {
    let session = detector.detect_or_use_provided(args.session.clone())?;
    
    let update = StatusUpdate {
        task,
        tests: args.tests.clone().unwrap_or_else(|| "unknown".to_string()),
        confidence: args.confidence.clone().unwrap_or_else(|| "medium".to_string()),
        todos: args.todos.clone(),
        blocked: args.blocked,
    };

    operations.update_status(state_dir, &session, update)?;
    Ok(())
}
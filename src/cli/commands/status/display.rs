use crate::core::status::Status;
use crate::utils::{ParaError, Result};

pub trait StatusDisplay {
    fn display(&self, status: &StatusInfo) -> Result<()>;
    fn display_all(&self, statuses: &[StatusInfo]) -> Result<()>;
}

#[derive(Debug)]
pub struct StatusInfo {
    pub status: Status,
}

impl StatusInfo {
    pub fn new(status: Status) -> Self {
        Self { status }
    }
}

pub struct JsonStatusDisplay;

impl StatusDisplay for JsonStatusDisplay {
    fn display(&self, status_info: &StatusInfo) -> Result<()> {
        let json_str = serde_json::to_string_pretty(&status_info.status).map_err(|e| {
            ParaError::config_error(format!("Failed to serialize status: {}", e))
        })?;
        println!("{}", json_str);
        Ok(())
    }

    fn display_all(&self, statuses: &[StatusInfo]) -> Result<()> {
        let statuses_vec: Vec<&Status> = statuses.iter().map(|s| &s.status).collect();
        let json_str = serde_json::to_string_pretty(&statuses_vec).map_err(|e| {
            ParaError::config_error(format!("Failed to serialize status: {}", e))
        })?;
        println!("{}", json_str);
        Ok(())
    }
}

pub struct HumanStatusDisplay;

impl StatusDisplay for HumanStatusDisplay {
    fn display(&self, status_info: &StatusInfo) -> Result<()> {
        display_status(&status_info.status);
        Ok(())
    }

    fn display_all(&self, statuses: &[StatusInfo]) -> Result<()> {
        if statuses.is_empty() {
            println!("No session statuses found.");
        } else {
            let status_vec: Vec<Status> = statuses.iter().map(|s| s.status.clone()).collect();
            display_all_statuses(&status_vec);
        }
        Ok(())
    }
}

fn display_status(status: &Status) {
    println!("Session: {}", status.session_name);
    println!("Task: {}", status.current_task);
    println!("Tests: {}", status.test_status);
    println!("Confidence: {}", status.confidence);

    if let Some(todos) = status.format_todos() {
        println!("Progress: {}", todos);
    }

    if status.is_blocked {
        println!("Status: BLOCKED");
        if let Some(reason) = &status.blocked_reason {
            println!("Reason: {}", reason);
        }
    }

    println!(
        "Last Update: {}",
        status.last_update.format("%Y-%m-%d %H:%M:%S UTC")
    );
}

fn display_all_statuses(statuses: &[Status]) {
    // Sort by last update time (most recent first)
    let mut sorted_statuses = statuses.to_vec();
    sorted_statuses.sort_by(|a, b| b.last_update.cmp(&a.last_update));

    println!(
        "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}",
        "Session", "Current Task", "Tests", "Confidence", "Progress", "Status"
    );
    println!("{}", "-".repeat(110));

    for status in sorted_statuses {
        let task = if status.current_task.len() > 38 {
            format!("{}...", &status.current_task[..35])
        } else {
            status.current_task.clone()
        };

        let progress = status.format_todos().unwrap_or_else(|| "-".to_string());
        let status_str = if status.is_blocked {
            "BLOCKED"
        } else {
            "Active"
        };

        println!(
            "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}",
            status.session_name,
            task,
            status.test_status.to_string(),
            status.confidence.to_string(),
            progress,
            status_str
        );
    }
}
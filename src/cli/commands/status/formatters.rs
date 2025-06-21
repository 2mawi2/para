use crate::core::status::Status;
use crate::utils::{ParaError, Result};

pub trait StatusFormatter {
    fn format_single(&self, status: &Status) -> Result<String>;
    fn format_multiple(&self, statuses: &[Status]) -> Result<String>;
}

pub struct JsonFormatter;

impl StatusFormatter for JsonFormatter {
    fn format_single(&self, status: &Status) -> Result<String> {
        serde_json::to_string_pretty(status).map_err(|e| {
            ParaError::config_error(format!("Failed to serialize status: {}", e))
        })
    }

    fn format_multiple(&self, statuses: &[Status]) -> Result<String> {
        serde_json::to_string_pretty(statuses).map_err(|e| {
            ParaError::config_error(format!("Failed to serialize status: {}", e))
        })
    }
}

pub struct TextFormatter;

impl StatusFormatter for TextFormatter {
    fn format_single(&self, status: &Status) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("Session: {}\n", status.session_name));
        output.push_str(&format!("Task: {}\n", status.current_task));
        output.push_str(&format!("Tests: {}\n", status.test_status));
        output.push_str(&format!("Confidence: {}\n", status.confidence));

        if let Some(todos) = status.format_todos() {
            output.push_str(&format!("Progress: {}\n", todos));
        }

        if status.is_blocked {
            output.push_str("Status: BLOCKED\n");
            if let Some(reason) = &status.blocked_reason {
                output.push_str(&format!("Reason: {}\n", reason));
            }
        }

        output.push_str(&format!(
            "Last Update: {}",
            status.last_update.format("%Y-%m-%d %H:%M:%S UTC")
        ));

        Ok(output)
    }

    fn format_multiple(&self, statuses: &[Status]) -> Result<String> {
        if statuses.is_empty() {
            return Ok("No session statuses found.".to_string());
        }

        // Sort by last update time (most recent first)
        let mut sorted_statuses = statuses.to_vec();
        sorted_statuses.sort_by(|a, b| b.last_update.cmp(&a.last_update));

        let mut output = String::new();
        output.push_str(&format!(
            "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}\n",
            "Session", "Current Task", "Tests", "Confidence", "Progress", "Status"
        ));
        output.push_str(&format!("{}\n", "-".repeat(110)));

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

            output.push_str(&format!(
                "{:<20} {:<40} {:<10} {:<10} {:<15} {:<10}\n",
                status.session_name,
                task,
                status.test_status.to_string(),
                status.confidence.to_string(),
                progress,
                status_str
            ));
        }

        Ok(output)
    }
}
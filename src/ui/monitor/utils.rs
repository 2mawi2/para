use chrono::{DateTime, Duration, Utc};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn format_activity(last_activity: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now - *last_activity;

    if duration < Duration::minutes(1) {
        "now".to_string()
    } else if duration < Duration::hours(1) {
        format!("{}m ago", duration.num_minutes())
    } else if duration < Duration::days(1) {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

pub fn truncate_task(task: &str, max_len: usize) -> String {
    if task.len() <= max_len {
        task.to_string()
    } else {
        format!("{}...", &task[..max_len - 3])
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_activity() {
        let now = chrono::Utc::now();

        // Test "now" (< 1 minute)
        let result = format_activity(&now);
        assert_eq!(result, "now");

        // Test minutes ago
        let five_minutes_ago = now - chrono::Duration::minutes(5);
        let result = format_activity(&five_minutes_ago);
        assert_eq!(result, "5m ago");

        // Test hours ago
        let two_hours_ago = now - chrono::Duration::hours(2);
        let result = format_activity(&two_hours_ago);
        assert_eq!(result, "2h ago");

        // Test days ago
        let three_days_ago = now - chrono::Duration::days(3);
        let result = format_activity(&three_days_ago);
        assert_eq!(result, "3d ago");
    }

    #[test]
    fn test_truncate_task() {
        // Test short task
        let short = "Short task";
        assert_eq!(truncate_task(short, 20), "Short task");

        // Test exact length
        let exact = "Exactly twenty chars";
        assert_eq!(truncate_task(exact, 20), "Exactly twenty chars");

        // Test truncation
        let long = "This is a very long task description that needs to be truncated";
        assert_eq!(truncate_task(long, 20), "This is a very lo...");
    }

    #[test]
    fn test_centered_rect() {
        use ratatui::layout::Rect;

        let screen = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 20, screen);

        // Should be centered
        assert_eq!(centered.x, 25); // (100 - 50) / 2
        assert_eq!(centered.y, 20); // (50 - 10) / 2
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 10);
    }
}

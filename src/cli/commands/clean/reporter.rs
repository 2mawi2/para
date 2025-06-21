use super::coordinator::CleanupResults;

pub struct CleanupReporter;

impl CleanupReporter {
    pub fn new() -> Self {
        Self
    }

    pub fn show_results(&self, results: &CleanupResults) {
        println!("🧹 Cleanup Complete");
        println!("==================\n");

        self.show_success_counts(results);
        self.show_errors(results);
        self.show_final_status(results);
    }

    fn show_success_counts(&self, results: &CleanupResults) {
        if results.stale_branches_removed > 0 {
            println!(
                "  ✅ Removed {} stale branches",
                results.stale_branches_removed
            );
        }

        if results.orphaned_state_files_removed > 0 {
            println!(
                "  ✅ Removed {} orphaned state files",
                results.orphaned_state_files_removed
            );
        }

        if results.old_archives_removed > 0 {
            println!(
                "  ✅ Removed {} old archived sessions",
                results.old_archives_removed
            );
        }
    }

    fn show_errors(&self, results: &CleanupResults) {
        if !results.errors.is_empty() {
            println!("\n⚠️  Some items couldn't be cleaned:");
            for error in &results.errors {
                println!("  • {}", error);
            }
        }
    }

    fn show_final_status(&self, results: &CleanupResults) {
        if self.is_nothing_cleaned(results) {
            println!("✨ Your Para environment was already clean!");
        }
    }

    fn is_nothing_cleaned(&self, results: &CleanupResults) -> bool {
        results.stale_branches_removed == 0
            && results.orphaned_state_files_removed == 0
            && results.old_archives_removed == 0
    }
}

impl Default for CleanupReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporter_creation() {
        let reporter = CleanupReporter::new();
        // Just verify it was created successfully
        let results = CleanupResults::default();
        reporter.show_results(&results);
    }

    #[test]
    fn test_is_nothing_cleaned_empty_results() {
        let reporter = CleanupReporter::new();
        let results = CleanupResults::default();

        assert!(reporter.is_nothing_cleaned(&results));
    }

    #[test]
    fn test_is_nothing_cleaned_with_removals() {
        let reporter = CleanupReporter::new();
        let results = CleanupResults {
            stale_branches_removed: 1,
            orphaned_state_files_removed: 0,
            old_archives_removed: 0,
            errors: Vec::new(),
        };

        assert!(!reporter.is_nothing_cleaned(&results));
    }

    #[test]
    fn test_default_reporter() {
        let reporter = CleanupReporter;
        let results = CleanupResults::default();
        reporter.show_results(&results);
    }
}

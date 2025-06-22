# Replace Confidence column with Git diff statistics in para monitor

## Goal
Replace the "Confidence" column in `para monitor` with a "Changes" column that shows git diff statistics (lines added/removed) for each session, similar to GitHub's diff display.

## Requirements

### 1. Modify status reporting
- Replace the confidence field with git diff statistics
- The para status command should calculate diff stats locally when reporting status
- Show format like "+123 -45" with appropriate colors (green for additions, red for deletions)

### 2. Git diff calculation
- Calculate diff from the session's current state to its parent branch
- Use the common method to find the parent git root reliably
- Compare against the branch the session was originally created from (stored in session state)

### 3. Implementation details
- Update `SessionStatus` struct to replace `confidence` with `diff_stats` field
- Add a method to calculate git diff statistics in the worktree
- Format: `DiffStats { additions: usize, deletions: usize }`
- Update monitor display to show colored diff stats instead of confidence

### 4. Edge case handling
- When git diff cannot be determined (e.g., no git repo, detached HEAD, etc.)
- Show "-" or "N/A" instead of throwing errors
- Handle cases where parent branch has been deleted
- Handle uncommitted changes vs committed changes appropriately

### 5. Display formatting
- Use terminal colors: green for additions (+), red for deletions (-)
- Compact format: "+123 -45" to fit in the column
- Ensure alignment with other columns in the monitor display

### 6. Testing requirements
- Test with various git states (clean, dirty, staged changes)
- Test edge cases (no git repo, deleted parent branch)
- Verify color output works correctly
- Ensure monitor display remains properly aligned

## Implementation notes
- The git diff should be calculated by `para status` command locally, not by the agent
- Use existing git utilities in the codebase for diff operations
- Maintain backwards compatibility with existing status reporting

When done: para finish "Replace confidence with git diff stats in monitor"
# Fix Git Diff Calculation for Status Command

## Problem
The Changes UI in para monitor always shows "+0 -0" even when there are actual changes. The diff calculation doesn't seem to work correctly when agents call the status command.

## Investigation Points
1. The `calculate_diff_stats` function in `src/core/git/diff.rs` seems to be implemented correctly
2. The status command in `src/cli/commands/status.rs` calls `calculate_diff_stats_for_session`
3. The monitor UI properly displays diff stats when they exist (see `src/ui/monitor/renderer.rs`)

## Task
1. Create a failing test that reproduces the issue where diff stats show as 0 even with changes
2. Debug why the diff calculation fails in real agent scenarios
3. Fix the implementation to ensure diff stats are properly calculated

## Potential Issues to Check
- Agents might not be in the correct worktree directory when calling status
- The base branch detection might be failing
- The git diff command might be failing silently
- There might be a race condition between file changes and diff calculation

## Testing Requirements
- Write comprehensive tests that verify diff calculation works in various scenarios:
  - With staged changes
  - With unstaged changes  
  - With both staged and unstaged changes
  - From different working directories
  - With different branch configurations

When done: para finish "Fix diff calculation showing 0 changes in monitor UI"
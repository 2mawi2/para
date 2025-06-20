# Technical Debt Management Plan

## Overview

A system to prevent duplicate technical debt issues and allow users to decline/accept specific issues, similar to Dependabot's approach.

## Key Components

### 1. Issue Tracking Database (.github/technical-debt-tracking.json)

Store metadata about all technical debt issues in a JSON file committed to the repo:

```json
{
  "ignored_issues": [
    {
      "fingerprint": "unwrap_in_file:src/core/session/recovery.rs:line:145",
      "title": "Unsafe unwrap in recovery.rs",
      "ignored_date": "2024-01-15",
      "reason": "Intentional panic for unrecoverable state"
    }
  ],
  "active_issues": [
    {
      "fingerprint": "duplicate_test_config:5_files",
      "issue_number": 123,
      "created_date": "2024-01-10"
    }
  ],
  "resolved_issues": [
    {
      "fingerprint": "missing_tests:src/cli/commands/cancel.rs",
      "issue_number": 120,
      "resolved_date": "2024-01-12",
      "pr_number": 125
    }
  ]
}
```

### 2. Issue Fingerprinting

Create unique fingerprints for each type of technical debt:
- **Unwrap/Expect**: `unwrap_in_file:{file}:line:{line_number}`
- **Missing Tests**: `missing_tests:{file_or_module}`
- **Code Duplication**: `duplicate:{description_hash}`
- **Complex Functions**: `complexity:{function_name}:{file}`
- **Dependencies**: `outdated_dep:{package_name}`

### 3. Workflow Updates

#### Auditor Workflow Enhancement

1. **Pre-Analysis Step**: 
   - Fetch existing technical-debt-tracking.json
   - Pass it to Claude in the prompt
   - Claude checks fingerprints before creating new issues

2. **Prompt Addition**:
   ```
   Before creating any issues, read the file .github/technical-debt-tracking.json
   This file contains:
   - ignored_issues: Do NOT create issues for these (user has declined them)
   - active_issues: Do NOT create duplicate issues (they already exist)
   - resolved_issues: Can be recreated if the problem reappears
   
   For each potential issue:
   1. Generate a fingerprint based on the issue type
   2. Check if fingerprint exists in ignored_issues (skip if found)
   3. Check if fingerprint exists in active_issues (skip if found)
   4. Only create issue if fingerprint is new or was previously resolved
   ```

3. **Post-Issue Creation**:
   - Update technical-debt-tracking.json with new active issues
   - Commit the updated file back to the repo

#### Issue Lifecycle Management

1. **When Issue is Closed**:
   - If closed as "not planned" → Move to ignored_issues
   - If closed as "completed" → Move to resolved_issues
   - Update via GitHub Actions on issue close event

2. **Manual Ignore Command**:
   - Users can comment `/ignore-technical-debt reason` on an issue
   - Workflow moves it to ignored_issues with the reason

### 4. Implementation Steps

1. **Create Initial Tracking File**:
   ```bash
   mkdir -p .github
   echo '{"ignored_issues":[],"active_issues":[],"resolved_issues":[]}' > .github/technical-debt-tracking.json
   ```

2. **Add GitHub Action for Issue Lifecycle**:
   Create `.github/workflows/technical-debt-tracker.yml`:
   - Triggers on: issues closed, issue comments
   - Updates technical-debt-tracking.json based on actions
   - Commits changes automatically

3. **Update Auditor Workflow**:
   - Add step to make tracking file available
   - Update Claude prompt with tracking logic
   - Add post-processing to update tracking file

### 5. User Commands

- **Ignore an issue**: Close with "not planned" label or comment `/ignore-technical-debt [reason]`
- **Reopen consideration**: Comment `/reconsider-technical-debt` (removes from ignored list)
- **View ignored issues**: Check .github/technical-debt-tracking.json in repo

### 6. Benefits

1. **No Duplicate Issues**: Auditor checks existing issues before creating new ones
2. **User Control**: Users can decline specific technical debt permanently
3. **Transparency**: All decisions tracked in version control
4. **Flexibility**: Can reconsider previously ignored issues
5. **Automation-Friendly**: Works seamlessly with CI/CD

### 7. Future Enhancements

- Web dashboard to view/manage technical debt
- Automatic cleanup of resolved issues after X days
- Priority scoring based on issue age and impact
- Integration with code review to catch new technical debt

## Example Workflow

1. Auditor finds unwrap at line 145 in recovery.rs
2. Generates fingerprint: `unwrap_in_file:src/core/session/recovery.rs:line:145`
3. Checks tracking file - not found in ignored or active
4. Creates issue #150
5. Updates tracking file with active issue
6. User reviews and closes as "not planned" 
7. Tracking workflow moves to ignored_issues
8. Next Auditor run skips this specific unwrap

This approach gives users full control while maintaining automation efficiency.
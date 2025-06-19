# Task 2: Add Autonomous Reviewer and Iteration Loop

## Overview
Enhance the autonomous system with a Reviewer agent that provides feedback on Gardener's work and creates a self-correction loop until the work meets quality standards.

## Prerequisites
Task 1 (Foundational Auditor & Gardener) must be fully implemented and working.

## Acceptance Criteria

1. Gardener creates **Draft** PRs instead of ready PRs
2. Draft PR creation triggers new `reviewer.yml` workflow
3. Reviewer reads task requirements and code changes, posts comment ending with `[REVIEW_PASSED]` or `[REVIEW_FAILED]`
4. If review passes: PR marked "Ready for Review", loop terminates
5. If review fails: Gardener workflow re-triggered to fix issues
6. Re-triggered Gardener uses reviewer feedback to fix work on same branch
7. Updated branch re-triggers Reviewer, continuing loop
8. Maximum 3 retries to prevent infinite loops

## Implementation Requirements

### Phase 1: Modify Gardener Workflow

**Update existing gardener.yml:**
1. **Modify PR Creation:**
   - Add `draft: true` to `peter-evans/create-pull-request` step
   
2. **Add New Trigger:**
   - Add `workflow_run` trigger listening for `reviewer.yml` completion
   
3. **New Job: iterate-on-task**
   - **Condition:** Only run if triggered by workflow_run AND reviewer conclusion was failure
   - **Steps:**
     - Checkout specific PR branch (`github.event.workflow_run.head_branch`)
     - Download `pr-review-feedback` artifact using `actions/download-artifact`
     - Run `claude-code-action` with iteration prompt:
       - "Your previous work failed review. You MUST fix your code based on the feedback in `pr_review_feedback.txt`. The original requirements are in `[task_file]`."
     - Agent works and runs `para finish` to commit fixes to same branch
     - Push changes (automatically updates PR and re-triggers Reviewer)

### Phase 2: Implement Reviewer Workflow

**Create .github/workflows/reviewer.yml:**

**Triggers:**
- `on: pull_request: types: [opened, synchronize]`

**Conditions:**
- Only run when `github.event.pull_request.draft == true`
- Only run when head branch starts with `gardener/`

**Job: review-pr**
1. **Checkout:** Specific PR commit (`github.event.pull_request.head.sha`)
2. **Extract Task:** Parse PR body to find original task file path
3. **Review Step:**
   - Use `grll/claude-code-action` to perform review
   - Prompt: "You are an expert code reviewer. Review the code changes in this PR against the requirements in `[task_file]`. Conclude your review with either `[REVIEW_PASSED]` or `[REVIEW_FAILED]` on a new line."
4. **Post Comment:** Use `gh pr comment` to post AI's full response
5. **Conditional Actions:**
   - **On Failure:** `if: contains(steps.claude_review.outputs.response, '[REVIEW_FAILED]')`
     - Write review response to `pr_review_feedback.txt`
     - Upload as artifact using `actions/upload-artifact` with name `pr-review-feedback`
     - Add "needs-revision" label to PR
   - **On Success:** `if: contains(steps.claude_review.outputs.response, '[REVIEW_PASSED]')`
     - Use `gh pr ready` to mark PR as ready for human review
     - Add "review-passed" label to PR

### Phase 3: Task Cleanup Workflow

**Create .github/workflows/cleanup-tasks.yml:**

**Triggers:**
- `on: pull_request: types: [closed]`

**Conditions:**
- Only run if `github.event.pull_request.merged == true`
- Only run if branch started with `gardener/`

**Job: cleanup-completed-task**
1. Checkout main branch
2. Parse merged PR body to find task file path
3. Use `git mv` to move task file from `tasks/gardener-backlog/` to `tasks/gardener-done/`
4. Commit and push change to main
5. **Requires:** `secrets.GH_PAT_FOR_ACTIONS` to push to main

### Phase 4: Loop Control and Safety

**Iteration Limits:**
- Track retry count in PR labels or comments
- Maximum 3 iterations before escalating to human review
- Add timeout protection for long-running reviews

**Error Handling:**
- Handle cases where task file is malformed
- Handle cases where reviewer response is ambiguous
- Graceful degradation when Claude API is unavailable

## Testing Strategy

### Local Testing with Act
1. Use `act` to test all three workflows locally
2. Create sample scenarios:
   - Successful review on first try
   - Failed review requiring iteration
   - Maximum retry scenarios
3. Test artifact passing between workflows
4. Verify proper branch and PR state management

### Integration Testing
1. Test complete flow from Auditor → Gardener → Reviewer → Iteration
2. Verify cleanup workflow properly archives completed tasks
3. Test edge cases: malformed tasks, API failures, git conflicts
4. Ensure no race conditions between workflows

## Success Criteria
- Draft PRs automatically reviewed and iterated until passing quality checks
- Human reviewers only see high-quality, pre-reviewed PRs
- System handles failures gracefully with retry limits
- Complete audit trail of all review iterations
- Proper cleanup and archiving of completed tasks

## Required Secrets
- `CLAUDE_ACCESS_TOKEN`
- `CLAUDE_REFRESH_TOKEN`
- `CLAUDE_EXPIRES_AT`
- `GH_PAT_FOR_ACTIONS` (for cleanup workflow)

When done: para finish "Add autonomous reviewer and iteration loop to gardener system" --branch feature/autonomous-gardener-reviewer
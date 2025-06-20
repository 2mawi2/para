# Task 1: Implement Foundational Autonomous Loop (Gardener)

## Overview
Implement a two-stage autonomous system for technical debt remediation using GitHub Actions workflows and para dispatch commands.

## Acceptance Criteria

1. Create a GitHub Actions workflow (`gardener.yml`) that runs on schedule to analyze codebase for technical debt
2. Gardener opens PR to add task files to `tasks/gardener-backlog/` directory  
3. When task proposal PR is merged, trigger `gardener.yml` workflow
4. Gardener creates feature branch and uses `para dispatch` to perform work
5. Gardener runs `para finish` to commit changes and create review branch
6. Gardener opens PR from feature branch to main with proposed fix
7. No "in-progress" or "lock" files committed to main branch

## Implementation Requirements

### Phase 1: Gardener Analysis Workflow (.github/workflows/gardener.yml)

**Triggers:**
- Schedule: `cron: '0 5 * * *'` (nightly)
- Manual: `workflow_dispatch`

**Job: propose-task**
1. Checkout repository
2. Install dependencies: rust, just, tools
3. Analysis step:
   - Use `rg --type rust -l "\.unwrap\(\)" src/` to find .unwrap() usage
   - Select random file: `shuf -n 1`
   - Generate unique task filename: `tasks/gardener-backlog/$(date +%s)-fix-unwrap-in-[sanitized-filename].md`
4. Task generation:
   - Use `grll/claude-code-action` with Write permission
   - Prompt Claude to create markdown task file specifying:
     - File to fix
     - Goal: Replace .unwrap() with proper ? error propagation
     - Required final command: `para finish "refactor: Replace unwrap in [filename]" --branch "gardener/fix-unwrap-in-[filename-sanitized]"`
5. PR creation:
   - Use `peter-evans/create-pull-request`
   - Title: "Gardener Proposal: New Tech Debt Task"
   - Branch: `gardener/propose-task-${{ GITHUB.RUN_ID }}`
   - Delete branch on merge

### Phase 2: Gardener Worker Workflow (.github/workflows/gardener-worker.yml)

**Triggers:**
- Push to main branch with changes in `tasks/gardener-backlog/` path only

**Job: work-on-task**
1. Checkout repository
2. Identify new task:
   - Use `git diff-tree --no-commit-id --name-only -r ${{ github.sha }}`
   - Find exact .md file added in triggering commit
3. Create branch:
   - Generate descriptive branch name from task filename
   - Checkout new `gardener/` branch
4. Dispatch agent:
   - Run `para dispatch` with:
     - Session name derived from task
     - `--file` argument reading from task file
     - `--dangerously-skip-permissions` flag
   - Command blocks until agent completes `para finish`
5. PR creation:
   - Use `peter-evans/create-pull-request`
   - From `gardener/` branch to main
   - Title: "🌿 Gardener Fix for: [task_file_name]"
   - Body includes reference to original task file path

### Phase 3: Testing with Act

Before deploying workflows:
1. Install `act` for local GitHub Actions testing
2. Test workflows locally using act
3. Use `just refresh-claude-secrets` pattern for credential management
4. Verify workflows work end-to-end in isolated environment
5. Test error conditions and edge cases

### Required Secrets
- `CLAUDE_ACCESS_TOKEN`
- `CLAUDE_REFRESH_TOKEN` 
- `CLAUDE_EXPIRES_AT`

## Success Criteria
- Main branch only modified when humans merge PRs
- All intermediate state exists on feature branches
- System identifies, proposes, and fixes technical debt autonomously
- Workflows are testable and reliable

## Testing Strategy
- Use `act` to test GitHub Actions locally before deployment
- Test with sample .unwrap() cases in codebase
- Verify proper branch creation and cleanup
- Ensure idempotent behavior when re-running workflows

When done: para finish "Implement foundational gardener autonomous loop" --branch feature/autonomous-gardener-foundation
# Decision: Keep "finish" Command Name

## Summary
After researching the naming of the `para finish` command, the decision is to **keep the current name** but update its description to be more accurate.

## Research Findings

### Current State
- **Misleading Description**: "Squash all changes into single commit" doesn't fully describe the command's behavior
- **Actual Behavior**: The command:
  - Stages all uncommitted changes
  - Creates a commit with the provided message
  - Creates or manages a feature branch (with optional `--branch` flag)
  - Removes the worktree (unless preserve mode is on)
  - Updates session status to "Review"

### Naming Analysis
1. **Para's Pattern**: Commands use action verbs (start, dispatch, finish, cancel, resume, recover)
2. **Git Conventions**: Mix of verbs and nouns (git branch, git commit, git checkout)
3. **Git Flow Precedent**: Uses "finish" for completing features (feature finish, release finish, hotfix finish)

### User Expectations
- **"finish"**: Correctly implies completing/ending a work session
- **"branch"**: Would suggest branch management operations, not session completion
- **Git Flow Alignment**: "finish" matches established patterns for completing feature work

## Decision Rationale

### Keep "finish" Because:
1. **Semantic Accuracy**: "finish" accurately conveys completing a work session
2. **Git Flow Consistency**: Aligns with `git flow feature finish` pattern
3. **Para's Design**: Maintains consistency with other action-oriented commands
4. **User Intuition**: Clear that you're completing your work, not just creating a branch

### Updated Description
Changed from: "Squash all changes into single commit"
Changed to: "Complete session and create feature branch for review"

This better reflects that the command:
- Completes the current work session
- Creates a commit with all changes
- Prepares a feature branch for review
- Cleans up the worktree environment

## Implementation
- Updated command description in `src/cli/parser.rs`
- Updated MCP server description in `mcp-server-ts/src/para-mcp-server.ts`
- Updated README.md to clarify the workflow
- No breaking changes - command behavior remains the same
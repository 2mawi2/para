# Squash Command

This command squash merges one or more feature branches into the current branch (typically main).

## Usage
```
/squash [branch-name1] [branch-name2] ...
```

## Instructions

When this command is invoked, perform the following steps:

### 1. Verify Current Branch
First, check that you're on the target branch (usually main):
```bash
git status
```

### 2. For Each Branch Provided
Process each branch sequentially:

#### a. Analyze Changes
```bash
git diff main...[branch-name]
```
Look at the changes to understand what was implemented.

#### b. Squash Merge
```bash
git merge --squash [branch-name]
```

#### c. Create Commit
Generate a concise commit message based on the changes:
- **Keep it short** - One line summary (50-72 chars)
- **Be descriptive** - What does this change do?
- **No attribution** - Don't mention who did it or branch names
- **Focus on the "what"** - Not the "how" or "why"

Examples of good commit messages:
- "Add audio preprocessing with 16kHz sampling and normalization"
- "Fix recording animation state transitions"
- "Implement voice activity detection for smart recording"
- "Update button positioning to avoid UI overlaps"

```bash
git commit -m "[concise-commit-message]"
```

#### d. Clean Up
After successful merge, clean up the branch:
```bash
# Try to cancel para session if it exists
para cancel [session-name] --force

# Delete local branch (might fail if worktree exists)
git branch -d [branch-name] 2>/dev/null || true

# Delete remote branch if it exists
git push origin --delete [branch-name] 2>&1 || true
```

### 3. Error Handling
- If merge conflicts occur, resolve them before committing
- If a branch doesn't exist, skip it and continue with others
- If para session exists, cancel it before deleting branch

### 4. Summary
After processing all branches, provide a brief summary of what was merged.

## Examples

### Single Branch
```
/squash para/fix-recording-animation
```
Result: Squash merges the fix-recording-animation branch with commit like "Fix recording animation state transitions"

### Multiple Branches
```
/squash para/implement-vad para/fix-audio-levels
```
Result: Squash merges both branches sequentially with appropriate commits for each

## Important Notes
- Always analyze the actual changes to create meaningful commit messages
- Don't include implementation details in commit messages
- Keep messages concise and focused on the user-visible change
- The commit message should make sense in the context of `git log --oneline`
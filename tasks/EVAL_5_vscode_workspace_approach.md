# Evaluate VS Code Multi-Root Workspace Approach

## Task
Analyze the feasibility of using VS Code's native multi-root workspace feature with integrated terminal management for Para's multi-instance Claude Code support.

## Evaluation Criteria

### 1. Technical Feasibility
- VS Code multi-root workspace capabilities
- Terminal panel management via tasks.json
- Extension API for terminal control
- Claude Code integration in terminals

### 2. Implementation Details
- Workspace file generation
- Dynamic folder addition
- Terminal group presentation
- Task automation on folder open

### 3. User Experience
- Single VS Code window convenience
- File explorer organization
- Terminal tab management
- Keyboard shortcuts and navigation

### 4. Integration with Para
- Minimal changes to existing code
- Workspace file management
- Session state tracking
- Incremental folder addition

### 5. Limitations
- Not true separate instances
- Shared VS Code settings
- Extension conflicts potential
- Terminal-only Claude interaction

### 6. Pros and Cons
- Native VS Code feature
- No additional dependencies
- Resource efficient
- Limited to terminal interface

## Deliverables
1. Detailed implementation plan
2. Workspace file structure examples
3. Terminal management strategy
4. Recommendation score (1-10)

Please create a detailed evaluation in a file called `evaluation_vscode_workspace.md`

## Prototype Implementation
Create a working prototype in the `prototypes/vscode-workspace/` directory that demonstrates:
1. Multi-root workspace file generation
2. Dynamic folder addition to workspace
3. Terminal/task management for Claude instances
4. Integration with existing Para dispatch flow

Test with 2-3 worktrees in one workspace. DO NOT run `para finish` - leave the implementation for review.
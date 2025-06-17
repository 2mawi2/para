# Evaluate Terminal Multiplexer Approach

## Task
Analyze the feasibility of using terminal multiplexers (tmux, screen, Windows Terminal tabs) for Para's multi-instance Claude Code support.

## Evaluation Criteria

### 1. Technical Feasibility
- tmux/screen availability across platforms
- Windows Terminal tabs API
- Session persistence and recovery
- Terminal emulation limitations

### 2. Integration with Para
- Launching VS Code within tmux panes
- Claude Code terminal interaction
- Session management complexity
- State synchronization

### 3. User Experience
- Terminal-based navigation
- VS Code GUI within terminal context
- Copy/paste and mouse support
- Learning curve for users

### 4. Implementation Complexity
- Platform-specific multiplexer choices
- Configuration management
- Error recovery scenarios
- Testing automation

### 5. Pros and Cons
- Lightweight solution
- SSH session compatibility
- Limited to terminal interface
- Power user oriented

## Deliverables
1. Implementation strategy
2. Platform compatibility analysis
3. User workflow examples
4. Recommendation score (1-10)

Please create a detailed evaluation in a file called `evaluation_terminal_multiplexer.md`

## Prototype Implementation
Create a working prototype in the `prototypes/terminal-mux/` directory that demonstrates:
1. tmux session creation and management
2. Launching VS Code in tmux panes/windows
3. Integration with Para's session management
4. Script to attach and navigate between sessions

Test with at least 2 concurrent sessions. DO NOT run `para finish` - leave the implementation for review.
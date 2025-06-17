# Evaluate Code-Server Web Dashboard Approach

## Task
Analyze the feasibility and implementation details of using code-server (VS Code in browser) with a web dashboard for Para's multi-instance Claude Code support.

## Evaluation Criteria

### 1. Technical Feasibility
- Check if code-server is commonly available or needs installation
- Assess port management for multiple instances
- Evaluate browser compatibility and performance
- Consider how Claude Code would work in web VS Code

### 2. Integration with Para
- How would this fit with current architecture?
- Changes needed to SessionManager and IdeManager
- State tracking for running servers
- Port allocation strategy

### 3. User Experience
- Browser tab management vs native app
- Performance with multiple instances
- Terminal integration in web VS Code
- File system access limitations

### 4. Implementation Complexity
- Estimate effort to implement
- New dependencies required
- Cross-platform considerations
- Error handling scenarios

### 5. Pros and Cons
- List major advantages
- Identify limitations
- Security considerations
- Resource usage

## Deliverables
1. Technical assessment report
2. Sample implementation outline
3. Risk analysis
4. Recommendation score (1-10)

Please create a detailed evaluation in a file called `evaluation_code_server.md`

## Prototype Implementation
Create a working prototype in the `prototypes/code-server/` directory that demonstrates:
1. Basic code-server launcher implementation
2. Web dashboard with tabs
3. Integration with Para's existing code
4. At least 2 instances running simultaneously

Test the prototype to ensure it works. DO NOT run `para finish` - leave the implementation for review.
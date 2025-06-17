# Evaluate Native Window Tabs Approach

## Task
Analyze the feasibility of using native OS window tabbing features (macOS tabs, Windows Sets) for Para's multi-instance Claude Code support.

## Evaluation Criteria

### 1. Technical Feasibility
- Platform-specific implementation requirements
- AppleScript/PowerShell automation reliability
- Window management API limitations
- Timing and synchronization challenges

### 2. Integration with Para
- Platform detection and conditional logic
- Fallback strategies for unsupported platforms
- Current VS Code/Cursor tab support
- Claude Code process management

### 3. User Experience
- Native feel and performance
- Tab switching and organization
- Window state persistence
- Multi-monitor support

### 4. Implementation Complexity
- Platform-specific code maintenance
- Testing across OS versions
- Edge cases (window not found, merge failures)
- Accessibility concerns

### 5. Cross-Platform Analysis
- macOS: Window > Merge All Windows
- Windows: Feasibility without Sets
- Linux: Window manager dependencies
- Consistency across platforms

### 6. Pros and Cons
- Native integration benefits
- Platform fragmentation issues
- Maintenance burden
- User control and preferences

## Deliverables
1. Platform compatibility matrix
2. Implementation approach per OS
3. Code samples for each platform
4. Recommendation score (1-10)

Please create a detailed evaluation in a file called `evaluation_native_tabs.md`

## Prototype Implementation
Create a working prototype in the `prototypes/native-tabs/` directory that demonstrates:
1. Platform detection and native tab merging
2. AppleScript/PowerShell automation scripts
3. Integration with Para's IdeManager
4. Launch 2-3 VS Code instances and merge them into tabs

Test the prototype on your current platform. DO NOT run `para finish` - leave the implementation for review.
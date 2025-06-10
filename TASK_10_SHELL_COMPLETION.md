# Task 10: Shell Completion System

## Objective
Implement comprehensive shell completion for all para commands, arguments, and context-aware suggestions to dramatically improve CLI usability.

## Key Requirements

### Command Behavior
- `para completion bash` - Generate bash completion script
- `para completion zsh` - Generate zsh completion script
- `para completion fish` - Generate fish completion script
- Dynamic completion for session names, branch names, and file paths

### Core Functionality
1. **Multi-Shell Support**: Generate completion scripts for major shells
2. **Dynamic Completion**: Context-aware suggestions based on current state
3. **Command Completion**: Complete all para subcommands and options
4. **Argument Completion**: Complete command-specific arguments
5. **File Path Completion**: Complete file paths for file-based arguments
6. **Session Completion**: Complete session names from current repository

### Implementation Files to Modify
- `src/cli/commands/completion.rs` - Main completion command implementation
- `src/cli/completion/mod.rs` - Completion system architecture
- `src/cli/completion/generators.rs` - Shell-specific completion generators
- `src/cli/completion/dynamic.rs` - Dynamic completion logic
- `src/cli/completion/context.rs` - Context-aware completion

### Expected Integration Points
- Use `clap_complete` crate for shell completion generation
- Use `SessionManager` for session name completion
- Use `GitService` for branch name completion
- Use file system utilities for path completion
- Integrate with existing CLI parser structure

### Completion Categories

#### Static Completion
- **Commands**: All para subcommands (start, finish, dispatch, etc.)
- **Options**: All command-line flags and options
- **Values**: Predefined option values (merge strategies, etc.)

#### Dynamic Completion
- **Session Names**: Active session names from current repository
- **Branch Names**: Git branch names from current repository
- **File Paths**: Files and directories for file-based arguments
- **Archive Names**: Archived session names for recovery commands
- **IDE Names**: Available/configured IDE names

#### Context-Aware Completion
- **Repository Context**: Only show completions when in git repository
- **Session Context**: Different completions based on current session
- **Command Context**: Command-specific completions
- **Argument Context**: Position-specific argument completions

### Shell-Specific Implementation

#### Bash Completion
- Generate bash completion script using clap_complete
- Support dynamic completion via bash functions
- Handle completion for complex argument structures
- Support completion installation to system directories

#### Zsh Completion
- Generate zsh completion script with full feature support
- Utilize zsh's advanced completion features
- Support completion descriptions and help text
- Handle complex argument patterns and subcommands

#### Fish Completion
- Generate fish completion script with native fish syntax
- Support fish's completion description system
- Handle dynamic completion with fish functions
- Integrate with fish's completion framework


### Dynamic Completion Logic
1. **Repository Detection**: Detect if user is in para repository
2. **Session Discovery**: Find active sessions for completion
3. **Context Analysis**: Determine completion context from command line
4. **Suggestion Generation**: Generate appropriate suggestions
5. **Filtering**: Filter suggestions based on partial input
6. **Formatting**: Format suggestions for target shell

### Completion Installation
- **Auto-Installation**: Detect shell and install completion automatically
- **Manual Installation**: Provide instructions for manual installation
- **System Integration**: Install to standard system completion directories
- **User Integration**: Install to user-specific completion directories
- **Activation Instructions**: Provide clear activation instructions

### Advanced Features
- **Fuzzy Matching**: Support fuzzy matching for partial completions
- **Completion Caching**: Cache expensive completions for performance
- **Completion Validation**: Validate completion suggestions
- **Custom Completions**: Allow custom completion extensions
- **Completion Testing**: Test completion generation and functionality

### Performance Optimization
- **Lazy Loading**: Only compute completions when needed
- **Caching Strategy**: Cache session and branch lists
- **Fast Paths**: Optimize common completion scenarios
- **Timeout Handling**: Handle slow git operations gracefully
- **Background Updates**: Update completion cache in background

### Success Criteria
- Generates working completion scripts for all supported shells
- Provides context-aware completion for session and branch names
- Completes all para commands and their arguments correctly
- Handles file path completion for file-based arguments
- Works efficiently even in large repositories
- Provides clear installation and activation instructions
- Supports both local and global installation methods

### Error Handling
- Handle case when not in git repository
- Handle case when git operations are slow or fail
- Handle case when session state is corrupted
- Handle case when completion cache is invalid
- Handle case when shell is not supported
- Provide helpful error messages with suggested fixes
- Gracefully degrade when advanced features are unavailable

### Installation and Setup
- **Detection**: Auto-detect user's shell environment
- **Generation**: Generate appropriate completion script
- **Installation**: Install script to appropriate location
- **Activation**: Provide shell-specific activation commands
- **Verification**: Verify completion is working correctly

## Testing Requirements
- **Unit Tests**: Write comprehensive unit tests for completion logic
- **Integration Tests**: Test completion generation for all shells
- **Dynamic Tests**: Test dynamic completion in various repository states
- **Performance Tests**: Test completion performance with large datasets
- **Shell Tests**: Test actual completion functionality in real shells
- **Installation Tests**: Test completion installation process
- **All tests must be GREEN** - Task is not complete until all tests pass

## Quality Requirements
- **Linting**: All clippy lints must pass (`just lint`)
- **Formatting**: Code must be properly formatted (`just fmt`)
- **Type Safety**: No compiler warnings or errors
- **Performance**: Completion should be fast and responsive

## Completion Process
1. Implement the shell completion system
2. Write and ensure all tests pass (`just test`)
3. Fix any linting issues (`just lint`)
4. Test completion in actual shell environments
5. **Execute `git diff` and review your changes thoroughly**
6. **Call `para finish "Implement comprehensive shell completion system"` to commit your work**

**IMPORTANT**: Task is only complete when ALL tests pass, linting is clean, and you have reviewed your git diff.
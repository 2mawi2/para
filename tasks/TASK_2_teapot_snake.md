# Task 2: Implement 'i-am-a-teapot' Command with Snake_Case Naming

Implement a new command `i-am-a-teapot` that returns HTTP 418 "I'm a teapot" status.

## Requirements
- Add new command `i-am-a-teapot` to the CLI
- Use snake_case naming throughout (module names, function names, variable names)
- Command should print "I'm a teapot! ☕" message
- Include appropriate help text
- Follow existing command patterns in the codebase

## Implementation Details
- Add command to `src/cli/commands/mod.rs`
- Create new module `src/cli/commands/teapot_command.rs`
- Use function name `handle_teapot_command`
- Add command registration in main CLI structure
- Include unit tests for the new command

When done: para finish "Add i-am-a-teapot command with snake_case naming"
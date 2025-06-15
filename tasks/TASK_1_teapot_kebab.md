# Task 1: Implement 'i-am-a-teapot' Command with Kebab-Case Naming

Implement a new command `i-am-a-teapot` that returns HTTP 418 "I'm a teapot" status.

## Requirements
- Add new command `i-am-a-teapot` to the CLI
- Use kebab-case naming throughout (command name, module names, function names with underscores as separators)
- Command should print "I'm a teapot! ☕" message
- Include appropriate help text
- Follow existing command patterns in the codebase

## Implementation Details
- Add command to `src/cli/commands/mod.rs`
- Create new module `src/cli/commands/i_am_a_teapot.rs` 
- Use function name `handle_i_am_a_teapot`
- Add command registration in main CLI structure
- Include unit tests for the new command

When done: para finish "Add i-am-a-teapot command with kebab-case naming"
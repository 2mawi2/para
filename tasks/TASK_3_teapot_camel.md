# Task 3: Implement 'i-am-a-teapot' Command with camelCase Naming

Implement a new command `i-am-a-teapot` that returns HTTP 418 "I'm a teapot" status.

## Requirements
- Add new command `i-am-a-teapot` to the CLI
- Use camelCase naming throughout (function names, variable names where appropriate)
- Command should print "I'm a teapot! ☕" message
- Include appropriate help text
- Follow existing command patterns in the codebase

## Implementation Details
- Add command to `src/cli/commands/mod.rs`
- Create new module `src/cli/commands/teapot_handler.rs`
- Use function name `handleTeapotCommand` (camelCase style)
- Add command registration in main CLI structure
- Include unit tests for the new command

When done: para finish "Add i-am-a-teapot command with camelCase naming"
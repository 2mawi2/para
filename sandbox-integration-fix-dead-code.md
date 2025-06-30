# Fix Dead Code Annotation in Sandbox Integration

The review identified a code style violation that needs to be fixed:

## Issue
In `src/core/sandbox/profiles.rs`, the `validate_profile_name` function is marked with `#[allow(dead_code)]`, which violates the project's code style requirement stated in CLAUDE.md: "Never allow dead code with `#[allow(dead_code)]`".

## Fix Required
1. Remove the `#[allow(dead_code)]` annotation from the `validate_profile_name` function
2. Either:
   - Use the function in the codebase where profile names need validation (recommended)
   - Remove the function entirely if it's not needed

## Implementation
Since this function validates profile names, it should be used in the `wrap_with_sandbox` function to ensure profile names are valid before use. This addresses one of the original review concerns about missing validation.

Update `src/core/sandbox/mod.rs` to use the validation function when wrapping commands with sandbox.

After implementing all fixes:
1. Commit all changes: `git add . && git commit -m 'Fix dead code annotation and add profile name validation'`
2. Verify build works: `just build`
3. Run: `para finish 'Fix dead code annotation in sandbox integration'`
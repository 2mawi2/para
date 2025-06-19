# Fix .unwrap() Usage in src/core/session/recovery.rs

## Task Description
Replace all `.unwrap()` calls in the file `src/core/session/recovery.rs` with proper error handling using the `?` operator.

## Requirements
1. Review the file `src/core/session/recovery.rs`
2. Identify all `.unwrap()` calls
3. Replace each `.unwrap()` with proper error propagation using `?`
4. Ensure the function signatures return appropriate `Result` types
5. Add proper error context where needed using `anyhow` or `thiserror`
6. Run `just test` to ensure all tests pass
7. Run `just lint` to ensure code quality

## File to Fix
`src/core/session/recovery.rs`

## Completion Command
When the task is complete, run:
```bash
para finish "refactor: Replace unwrap in recovery" --branch "gardener/fix-unwrap-in-recovery"
```

## Success Criteria
- All `.unwrap()` calls removed from the target file
- Proper error handling implemented
- All tests pass (`just test`)
- Code passes linting (`just lint`)
- No breaking changes to public APIs
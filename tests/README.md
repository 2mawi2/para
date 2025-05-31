# Para Tests

This directory contains the test suite for para using [bats-core](https://github.com/bats-core/bats-core).

## Running Tests

### With Just
```bash
just test
```

### Directly with Bats
```bash
bats tests/
```

### Single Test File
```bash
bats tests/test_para.bats
```

## Test Structure

- `test_para.bats` - Main test suite covering basic functionality
  - Script existence and permissions
  - Library module availability
  - Command-line interface validation
  - Basic command functionality
  - Syntax validation

## Writing Tests

Tests are written using bats syntax:

```bash
@test "description of test" {
    run command_to_test
    [ "$status" -eq 0 ]  # Check exit status
    [[ "$output" =~ "expected text" ]]  # Check output
}
```

## Test Environment

Each test runs in a clean environment with:
- Git repository initialized (if needed)
- Test user configuration
- Isolated from actual para sessions

## Adding New Tests

1. Add new test functions to existing `.bats` files
2. Or create new `.bats` files in this directory
3. Follow the naming convention: `test_*.bats`
4. Use descriptive test names

## Dependencies

Tests require:
- bats-core
- git
- The para scripts and libraries

Install with: `just install-dev` 
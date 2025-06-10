# TASK 23: GitHub Actions Rust Test Integration

## Overview

Update GitHub Actions test workflow to use Rust-focused testing pattern, replacing the current shell-based testing with proper Rust toolchain and testing approach.

## Dependencies

**Prerequisites:** TASK 20 (Shell to Rust Migration) - Requires Rust codebase in root
**Must complete before:** TASK 24 (Dependabot) - Dependabot needs test workflow to handle PRs
**Can be done in parallel with:** TASK 21

## Current State Analysis

### Current Test Workflow (.github/workflows/test.yml)
```yaml
name: Test

on:
  push:
    branches: [ main, master, develop ]
  pull_request:
    branches: [ main, master, develop ]

jobs:
  test:
    runs-on: macos-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Configure Git and Environment
      run: |
        # Configure git properly for CI environment
        git config --global user.email "test@example.com"
        git config --global user.name "GitHub Actions"
        # ... shell-specific setup
    
    - name: Install dependencies (macOS)  
      if: runner.os == 'macOS'
      run: |
        brew install shellcheck bats-core shfmt
    
    - name: Run shellcheck
      run: |
        shellcheck -e SC1091,SC2086 para.sh install-para.sh lib/*.sh
    
    - name: Run tests
      env:
        IDE_NAME: cursor
        IDE_CMD: echo
        CURSOR_CMD: echo
        PARA_NON_INTERACTIVE: true
      run: |
        echo "🧪 Running all para tests..."
        bats tests/test_para_units.bats
        bats tests/test_para_prompt_features.bats
        # ... more shell tests
```

### Issues with Current Approach
1. Shell-specific tooling (shellcheck, bats, shfmt)
2. Complex environment variable setup for shell testing
3. Legacy test dependencies
4. No Rust toolchain or testing
5. Mixed testing approaches

## New Rust-Focused Approach

### Updated GitHub Actions Workflow

Following the provided pattern but adapted for para:

```yaml
name: Rust Tests

on:
  push:
    branches: [ "main", "master", "release" ]
  pull_request:
    branches: [ "main", "master" ]
  pull_request_target:
    branches: [ "main", "master" ]
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: read

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test Para on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    if: |
      (github.event_name == 'push') ||
      (github.event_name == 'pull_request') ||
      (github.event_name == 'pull_request_target' && github.actor == 'dependabot[bot]')
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest]
    
    steps:
    - name: Checkout code
      uses: actions/checkout@v4.2.2
      with:
        ref: ${{ github.event_name == 'pull_request_target' && github.event.pull_request.head.sha || github.sha }}
        fetch-depth: 0
    
    - name: Set up Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1.12.0
      with:
        toolchain: stable
        cache: true
        components: clippy, rustfmt
    
    - name: Install system dependencies (macOS)
      if: matrix.os == 'macos-latest'
      run: |
        # Install git if needed (usually pre-installed)
        echo "macOS runner setup complete"

    - name: Install system dependencies (Ubuntu)
      if: matrix.os == 'ubuntu-latest'
      run: |
        sudo apt-get update
        sudo apt-get install -y git
        echo "Ubuntu runner setup complete"

    - name: Check Rust formatting
      run: cargo fmt --all -- --check

    - name: Run Clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Build
      run: cargo build --verbose

    - name: Run Tests
      run: cargo test --verbose --all-features

    - name: Test Binary Functionality
      run: |
        # Build and test basic functionality
        cargo build --release
        ./target/release/para --help
        ./target/release/para --version
```

## Implementation Tasks

### 1. Replace Current Test Workflow

#### 1.1 Update Workflow File
- Replace `.github/workflows/test.yml` with Rust-focused version
- Add proper Rust toolchain setup
- Include clippy and rustfmt checks
- Add binary functionality testing

#### 1.2 Configure Matrix Testing
- Test on macOS and Ubuntu (following current pattern but simplified)
- Remove Windows (as specified in requirements)
- Use latest stable Rust toolchain

#### 1.3 Add Rust-Specific Checks
- Formatting check with `cargo fmt`
- Linting with `cargo clippy`
- Comprehensive test execution with `cargo test`
- Binary functionality verification

### 2. Environment Configuration

#### 2.1 Simplify Environment Setup
- Remove shell-specific environment variables
- Add Rust-specific environment variables (CARGO_TERM_COLOR, RUST_BACKTRACE)
- Remove complex git configuration (use defaults)

#### 2.2 Update Permissions
- Set appropriate permissions for pull request testing
- Handle Dependabot PRs correctly
- Ensure security for pull_request_target events

### 3. Testing Strategy

#### 3.1 Focus on Rust Testing
- Remove all shell script testing (bats, shellcheck, shfmt)
- Add comprehensive Rust unit tests
- Add integration tests for CLI functionality
- Test actual binary execution

#### 3.2 Add Binary Validation
- Build release binary
- Test help and version commands
- Validate basic CLI functionality
- Ensure no runtime errors

### 4. Dependency Management

#### 4.1 Remove Shell Dependencies
- Remove shellcheck installation
- Remove bats-core installation  
- Remove shfmt installation
- Focus only on Rust toolchain

#### 4.2 Optimize Build Process
- Use Rust cache for faster builds
- Enable all features during testing
- Use verbose output for debugging

## Workflow Features

### Pull Request Handling
- Support for regular pull requests
- Handle Dependabot pull requests with pull_request_target
- Proper permission management
- Secure handling of external contributions

### Multi-Platform Support
- Test on macOS (primary target)
- Test on Ubuntu (secondary target)
- Platform-specific dependency installation
- Consistent Rust toolchain across platforms

### Comprehensive Testing
- Format checking (cargo fmt)
- Linting (cargo clippy with warnings as errors)
- Unit testing (cargo test)
- Binary functionality testing
- All features enabled during testing

## Comparison with Current Approach

### Removed Components
- Shell script linting (shellcheck)
- Shell script formatting (shfmt)
- Shell script testing (bats)
- Complex environment variable setup
- Git configuration overhead

### Added Components
- Rust formatting checks
- Rust linting with clippy
- Rust unit and integration tests
- Binary functionality validation
- Rust-specific caching

### Simplified Approach
- Standard Rust toolchain setup
- Cleaner environment configuration
- Focus on actual functionality testing
- Better CI/CD performance

## Testing Strategy

### Local Testing
```bash
# Test formatting
cargo fmt --all -- --check

# Test linting
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --verbose --all-features

# Test binary
cargo build --release
./target/release/para --help
```

### CI Testing Verification
- All Rust checks pass
- Binary builds successfully
- Basic CLI functionality works
- No warnings or errors in build process

## Completion Criteria

- [ ] GitHub Actions workflow updated to Rust-focused testing
- [ ] Shell script testing removed completely
- [ ] Rust formatting checks integrated (cargo fmt)
- [ ] Rust linting checks integrated (cargo clippy)
- [ ] Comprehensive Rust testing (cargo test)
- [ ] Binary functionality testing included
- [ ] Multi-platform testing (macOS and Ubuntu)
- [ ] Dependabot integration working correctly
- [ ] CI performance improved over shell-based testing
- [ ] All tests pass consistently

## Agent Instructions

1. **Backup Current Workflow**: Save existing test.yml before replacement
2. **Test Incrementally**: Verify each step of new workflow works
3. **Validate Rust Toolchain**: Ensure Rust setup works correctly
4. **Test Binary Execution**: Verify built binary functions properly
5. **Check All Platforms**: Test workflow on both macOS and Ubuntu
6. **Verify Pull Requests**: Test workflow with actual pull request
7. **Call `para finish 'Update GitHub Actions to Rust-focused testing'`** when completed
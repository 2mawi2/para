# TASK 24: Dependabot Cargo Integration

## Overview

Update Dependabot configuration to properly handle Rust/Cargo dependencies instead of shell-based dependencies, enabling automatic dependency updates for the Rust implementation.

## Dependencies

**Prerequisites:** 
- TASK 20 (Shell to Rust Migration) - Requires Cargo.toml in root
- TASK 23 (GitHub Actions Rust Tests) - Dependabot PRs need test workflow
**Must complete before:** None - this is the final task
**Can be done in parallel with:** None - depends on both TASK 20 and TASK 23

## Current State Analysis

### Current Dependabot Configuration
Currently, there may be no `.github/dependabot.yml` file, or it might be configured for other package ecosystems. The project needs proper Cargo dependency management.

### Requirements for Rust Project
- Cargo dependency updates for Rust crates
- GitHub Actions dependency updates
- Proper scheduling and limits
- Integration with new Rust-focused CI/CD

## New Cargo-Focused Dependabot Configuration

### Updated Dependabot Configuration

Following the provided pattern for Cargo and GitHub Actions:

```yaml
version: 2
updates:
  # Enable version updates for Cargo dependencies
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
      time: "09:00"
    open-pull-requests-limit: 5
    labels:
      - "dependencies"
      - "rust"
    commit-message:
      prefix: "cargo"
      prefix-development: "cargo-dev"
      include: "scope"

  # Enable version updates for GitHub Actions
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
      day: "monday"
      time: "09:00"
    open-pull-requests-limit: 3
    labels:
      - "dependencies"
      - "github-actions"
    commit-message:
      prefix: "ci"
      include: "scope"
```

## Implementation Tasks

### 1. Create/Update Dependabot Configuration

#### 1.1 Add Cargo Ecosystem Support
- Monitor `Cargo.toml` and `Cargo.lock` files
- Set up weekly updates for Rust dependencies
- Configure reasonable pull request limits
- Add appropriate labels for organization

#### 1.2 Add GitHub Actions Support
- Monitor `.github/workflows/` directory
- Update GitHub Actions versions automatically
- Separate scheduling from Cargo updates
- Lower pull request limit for actions

#### 1.3 Configure Update Scheduling
- Weekly updates on Monday mornings
- Avoid overwhelming with daily updates
- Coordinate with release schedule
- Minimize disruption to development workflow

### 2. Dependency Categories

#### 2.1 Cargo Dependencies
Monitor these dependency types:
- **Core dependencies**: clap, serde, anyhow, directories, etc.
- **Development dependencies**: tempfile, etc.
- **Build dependencies**: Any build-time crates
- **Platform-specific dependencies**: Any conditional dependencies

#### 2.2 GitHub Actions Dependencies
Monitor these action updates:
- **Checkout actions**: actions/checkout
- **Rust toolchain**: actions-rust-lang/setup-rust-toolchain
- **Cache actions**: Swatinem/rust-cache
- **Release actions**: softprops/action-gh-release

### 3. Configuration Options

#### 3.1 Update Limits
- **Cargo**: 5 open PRs maximum (more dependencies expected)
- **GitHub Actions**: 3 open PRs maximum (fewer actions to update)
- Prevents overwhelming the repository with update PRs
- Allows focus on important updates

#### 3.2 Labeling Strategy
- **"dependencies"**: Common label for all dependency updates
- **"rust"**: Specific to Cargo/Rust dependency updates
- **"github-actions"**: Specific to CI/CD action updates
- Enables easy filtering and organization

#### 3.3 Commit Message Format
- **Cargo**: Prefixed with "cargo" for consistency
- **GitHub Actions**: Prefixed with "ci" for CI-related changes
- Include scope information for better commit history
- Consistent with conventional commit standards

### 4. Integration with CI/CD

#### 4.1 Automatic Testing
- Dependabot PRs trigger Rust test workflow
- All Cargo dependency updates tested automatically
- GitHub Actions updates validated in CI
- Pull request target handling for security

#### 4.2 Review Process
- Dependabot PRs can be auto-merged if tests pass
- Security updates prioritized
- Breaking changes require manual review
- Integration with GitHub's security advisories

## Advanced Configuration Options

### Security Updates
```yaml
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "daily"  # More frequent for security
    open-pull-requests-limit: 10
    labels:
      - "dependencies"
      - "security"
      - "rust"
    assignees:
      - "maintainer-username"  # Optional: assign security updates
```

### Ignore Specific Dependencies
```yaml
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    ignore:
      - dependency-name: "some-crate"
        versions: ["1.x"]  # Ignore major version updates
```

### Commit Message Customization
```yaml
    commit-message:
      prefix: "deps"
      prefix-development: "deps-dev"
      include: "scope"
```

## Benefits of Cargo Integration

### Automated Dependency Management
- **Security updates**: Automatic security vulnerability fixes
- **Feature updates**: Access to new crate features and improvements
- **Performance updates**: Benefit from performance improvements in dependencies
- **Bug fixes**: Automatic bug fixes from upstream crates

### Development Workflow
- **Reduced manual work**: No need to manually check for updates
- **Consistent updates**: Regular, scheduled dependency maintenance
- **Testing integration**: All updates automatically tested
- **Easy review**: Clear PR format for dependency changes

### Project Health
- **Up-to-date dependencies**: Avoid technical debt from outdated dependencies
- **Security posture**: Faster response to security vulnerabilities
- **Compatibility**: Stay current with Rust ecosystem changes
- **Maintenance**: Reduce long-term maintenance burden

## File Structure

### Create Dependabot Configuration
```
.github/
├── dependabot.yml          # New Dependabot configuration
└── workflows/
    ├── test.yml            # Updated to handle Dependabot PRs
    └── release.yml         # Release workflow
```

### Cargo Files to Monitor
```
/
├── Cargo.toml              # Main dependencies
├── Cargo.lock              # Locked versions
└── src/                    # Source code using dependencies
```

## Testing Strategy

### Local Validation
```bash
# Check Dependabot config syntax
gh api repos/:owner/:repo/dependency-graph/snapshots --method POST

# Validate Cargo.toml
cargo check

# Test with dependency updates
cargo update
cargo test
```

### Integration Testing
- Create test PR to verify Dependabot workflow
- Verify labels are applied correctly
- Check commit message format
- Ensure CI triggers properly

## Completion Criteria

- [ ] Dependabot configuration file created (`.github/dependabot.yml`)
- [ ] Cargo ecosystem monitoring enabled
- [ ] GitHub Actions ecosystem monitoring enabled
- [ ] Appropriate update schedules configured
- [ ] Pull request limits set correctly
- [ ] Labels configured for easy organization
- [ ] Commit message format standardized
- [ ] Integration with Rust test workflow verified
- [ ] Security update handling configured
- [ ] Documentation updated for dependency management

## Monitoring and Maintenance

### Regular Review
- Weekly review of Dependabot PRs
- Monitor for failed updates
- Adjust configuration based on update volume
- Review security advisory integration

### Troubleshooting
- Check Dependabot logs for failed updates
- Verify CI integration works correctly
- Adjust limits if too many/few PRs created
- Update ignore lists for problematic dependencies

## Agent Instructions

1. **Check Current State**: Verify if `.github/dependabot.yml` exists
2. **Create Configuration**: Use provided pattern as template
3. **Validate Syntax**: Ensure YAML syntax is correct
4. **Test Integration**: Verify it works with current CI setup
5. **Monitor First Run**: Check that Dependabot creates appropriate PRs
6. **Adjust if Needed**: Fine-tune configuration based on initial results
7. **Call `para finish 'Configure Dependabot for Cargo dependency management'`** when completed
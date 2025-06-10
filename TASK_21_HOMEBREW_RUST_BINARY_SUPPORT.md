# TASK 21: Homebrew Rust Source-Based Installation

## Overview

Update the Homebrew formula and GitHub Actions to use Cargo-based source installation instead of shell scripts, following the same pattern as the provided GitHub Actions example from the switchr project.

## Dependencies

**Prerequisites:** TASK 20 (Shell to Rust Migration) - Requires Rust codebase in root
**Must complete before:** None - can be done independently after TASK 20
**Can be done in parallel with:** TASK 23, TASK 24

## Current State Analysis

### Current Homebrew Formula (from GitHub Actions)
```ruby
class Para < Formula
  desc "Parallel IDE workflow helper for Git worktrees"
  homepage "https://github.com/2mawi2/para"
  url "https://github.com/2mawi2/para/archive/refs/tags/#{tag}.tar.gz"
  sha256 "#{SHA256}"
  license "MIT"
  version "#{version}"

  def install
    # Install the para script and libraries
    libexec.install "para.sh"
    libexec.install "lib"
    
    # Create wrapper script
    (bin/"para").write <<~EOS
      #!/usr/bin/env sh
      exec "#{libexec}/para.sh" "$@"
    EOS
    
    # Make wrapper executable
    chmod 0755, bin/"para"
  end

  test do
    system "#{bin}/para", "--help"
  end
end
```

### Issues with Current Approach
1. Downloads entire source code including shell scripts
2. Requires shell interpreter at runtime
3. No optimization or compilation
4. Platform-specific shell compatibility issues
5. Slower startup times due to script interpretation

## New Cargo-Based Approach

Following the same pattern as the provided switchr example, we'll use source-based installation with Cargo.

### Updated GitHub Actions Workflow

#### 1. Version Extraction (matches switchr pattern)
```yaml
extract-version:
  name: Extract Version
  runs-on: ubuntu-latest
  outputs:
    version: ${{ steps.version.outputs.version }}
    tag: ${{ steps.version.outputs.tag }}
  steps:
    - name: Checkout Code
      uses: actions/checkout@v4
      
    - name: Extract current version
      id: version
      shell: bash
      run: |
        # Extract current version from Cargo.toml
        CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
        echo "version=$CURRENT_VERSION" >> $GITHUB_OUTPUT
        echo "tag=v$CURRENT_VERSION" >> $GITHUB_OUTPUT
        echo "Current version: $CURRENT_VERSION"
```

#### 2. Build Job (simplified, no pre-compiled binaries)
```yaml
build:
  name: Test Build
  needs: [extract-version]
  runs-on: ubuntu-latest
  steps:
    - name: Checkout Code
      uses: actions/checkout@v4
      with:
        fetch-depth: 0

    - name: Set up Rust
      uses: dtolnay/rust-toolchain@stable
    
    - name: Set up Rust Cache
      uses: Swatinem/rust-cache@v2
    
    - name: Test build
      run: cargo build --release
```

#### 3. Updated Homebrew Formula Generation (cargo install based)
```yaml
update-homebrew:
  needs: [extract-version, build, create-release]
  runs-on: ubuntu-latest
  if: always() && needs.build.result == 'success' && needs.create-release.result == 'success'
  steps:
    - name: Checkout homebrew tap repository
      uses: actions/checkout@v4
      with:
        repository: 2mawi2/homebrew-tap
        token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
        path: homebrew-tap
        
    - name: Download release tarball and calculate SHA
      run: |
        curl -L https://github.com/2mawi2/para/archive/refs/tags/${{ needs.extract-version.outputs.tag }}.tar.gz -o para.tar.gz
        SHA256=$(sha256sum para.tar.gz | awk '{print $1}')
        echo "SHA256=$SHA256" >> $GITHUB_ENV
        
    - name: Update formula
      run: |
        cat > homebrew-tap/Formula/para.rb << EOL
        class Para < Formula
          desc "Parallel IDE workflow helper for Git worktrees"
          homepage "https://github.com/2mawi2/para"
          url "https://github.com/2mawi2/para/archive/refs/tags/${{ needs.extract-version.outputs.tag }}.tar.gz"
          sha256 "${SHA256}"
          license "MIT"
        
          depends_on "rust" => :build
        
          def install
            system "cargo", "install", *std_cargo_args
          end
        
          test do
            assert_match "para", shell_output("#{bin}/para --version")
          end
        end
        EOL
```

### New Formula Structure

#### Cargo-Based Formula (following switchr pattern)
```ruby
class Para < Formula
  desc "Parallel IDE workflow helper for Git worktrees"
  homepage "https://github.com/2mawi2/para"
  url "https://github.com/2mawi2/para/archive/refs/tags/v1.0.0.tar.gz"
  sha256 "source_tarball_checksum_here"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  # Optional: Add shell completion
  def caveats
    <<~EOS
      To enable shell completion, add this to your shell config:
      
      For bash:
        echo 'eval "$(para completion bash)"' >> ~/.bashrc
      
      For zsh:
        echo 'eval "$(para completion zsh)"' >> ~/.zshrc
      
      For fish:
        para completion fish | source
    EOS
  end

  test do
    assert_match "para", shell_output("#{bin}/para --version")
    system "#{bin}/para", "--help"
  end
end
```

## Implementation Tasks

### 1. Update GitHub Actions Release Workflow

#### 1.1 Add Rust Toolchain Setup
- Install Rust toolchain in CI environment
- Set up caching for faster builds
- Remove cross-compilation (source-based installation handles this)

#### 1.2 Simplify Build Process
- Remove binary build matrix (not needed for source-based)
- Add test build to verify compilation works
- Focus on source tarball generation

#### 1.3 Update Release Asset Management
- Keep source tarball as main asset
- Generate checksums for source tarball
- Test source compilation in CI

### 2. Update Homebrew Formula Generation

#### 2.1 Modify Formula Structure
- Change to cargo install based approach
- Add Rust as build dependency
- Use standard cargo args for installation
- Remove platform-specific logic (Cargo handles this)

#### 2.2 Enhance Testing
- Test cargo installation process
- Verify command-line interface
- Test basic functionality
- Add version checking

#### 2.3 Add Shell Completion Support
- Generate completion scripts with Rust binary
- Provide installation instructions
- Test completion functionality

### 3. Version Management (following switchr pattern)

#### 3.1 Add Version Increment Step
- Extract current version from Cargo.toml
- Auto-increment version after successful release
- Commit version bump back to release branch
- Merge to main branch

#### 3.2 Release Branch Workflow
- Trigger releases from release branch pushes
- Handle manual workflow dispatch
- Ensure tests pass before release

### 4. Quality Assurance

#### 4.1 Formula Testing
- Test formula installation on clean macOS systems
- Verify cargo installation works correctly
- Test on both Intel and Apple Silicon Macs
- Validate shell completion installation

#### 4.2 Performance Validation
- Measure compilation time during installation
- Verify runtime performance improvements
- Test with real-world workflows
- Document performance gains

## Testing Strategy

### Local Testing
```bash
# Test binary build locally
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Test formula locally
brew install --build-from-source ./Formula/para.rb
brew test para
brew uninstall para

# Test completion
para completion bash > /tmp/para_completion
source /tmp/para_completion
```

### CI Testing
- Automated formula validation
- Binary functionality testing
- Cross-platform compatibility testing
- Integration testing with real repositories

## Benefits of Cargo-Based Distribution

### Performance Improvements
- **Faster startup**: No shell script interpretation overhead
- **Better resource usage**: Compiled code uses less memory and CPU
- **Optimized builds**: Cargo handles platform-specific optimizations
- **Native performance**: Compiled specifically for target architecture

### Distribution Advantages
- **Standard Rust workflow**: Follows Rust ecosystem conventions
- **Automatic optimization**: Cargo optimizes for local architecture
- **Dependency management**: Cargo handles all Rust dependencies
- **Build caching**: Homebrew can cache Cargo builds efficiently

### Maintenance Benefits
- **Simple formula**: Standard cargo install approach
- **Better testing**: Source compilation verifies build process
- **Reduced complexity**: No pre-built binary management
- **Standard tooling**: Uses standard Rust/Cargo toolchain

## Implementation Steps

1. **Update GitHub Actions**: Modify release workflow to follow switchr pattern
2. **Test Cargo Build**: Verify cargo build works correctly
3. **Update Formula**: Modify Homebrew formula for cargo-based installation  
4. **Test Formula**: Validate formula installation and functionality
5. **Add Version Management**: Implement auto-increment following switchr pattern
6. **Deploy Changes**: Release new version with cargo support
7. **Monitor**: Watch for installation issues and user feedback
8. **Document**: Update README and documentation for new installation method

## Completion Criteria

- [ ] GitHub Actions follows switchr workflow pattern
- [ ] Version extraction from Cargo.toml works correctly
- [ ] Homebrew formula uses cargo install approach
- [ ] Formula correctly builds from source tarball
- [ ] Source-based installation is reliable and consistent
- [ ] All existing functionality works with installed binary
- [ ] Shell completion works with cargo-installed binary
- [ ] Formula passes all Homebrew tests
- [ ] Version auto-increment works correctly
- [ ] Documentation updated for new installation method

## Agent Instructions

1. **Follow switchr Pattern**: Use the provided GitHub Actions workflow as template
2. **Test Incrementally**: Test each change in isolation before combining  
3. **Verify Source Builds**: Ensure cargo build works from source tarball
4. **Test Formula**: Test formula installation multiple times
5. **Validate Version Management**: Ensure version increment workflow works
6. **Run All Tests**: Ensure existing functionality still works after changes
7. **Call `para finish 'Update Homebrew to support Cargo installation'`** when completed
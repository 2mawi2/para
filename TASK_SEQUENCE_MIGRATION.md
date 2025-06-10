# Task Sequence: Shell to Rust Migration

## Overview

This document outlines the correct sequence for migrating para from shell-based to Rust implementation, ensuring no feature overlaps and proper dependencies.

## Task Execution Order

### 1. TASK 20: Shell to Rust Migration (Foundation)
**Status:** Must be completed first
**Dependencies:** None
**Purpose:** Core code migration and structure changes

**Key Responsibilities:**
- Move Rust code from `para-rs/` to root directory
- Remove all shell scripts and lib/ directory  
- Update justfile for Rust commands
- Implement shell completion with clap_complete
- Ensure binary builds and functions correctly
- Clean up directory structure

**Outputs:** 
- Working Rust binary at root level
- Cargo.toml configured for releases
- Clean codebase ready for CI/CD integration

---

### 2A. TASK 21: Homebrew Integration (Parallel)
**Status:** After TASK 20, can run in parallel with TASK 23
**Dependencies:** TASK 20 completed
**Purpose:** Distribution and release automation

**Key Responsibilities:**
- Update GitHub Actions release workflow (following switchr pattern)
- Modify Homebrew formula for cargo-based installation
- Add version management and auto-increment
- Test formula installation process

**Outputs:**
- Updated `.github/workflows/release.yml`
- Cargo-based Homebrew formula
- Automated release process

---

### 2B. TASK 23: GitHub Actions Test Integration (Parallel)  
**Status:** After TASK 20, can run in parallel with TASK 21
**Dependencies:** TASK 20 completed
**Purpose:** CI/CD test workflow modernization

**Key Responsibilities:**
- Replace shell-based tests with Rust testing
- Update `.github/workflows/test.yml` with Rust toolchain
- Add cargo fmt, clippy, and test integration
- Handle Dependabot PR testing

**Outputs:**
- Updated `.github/workflows/test.yml`
- Rust-focused CI pipeline
- Dependabot-ready test workflow

---

### 3. TASK 24: Dependabot Integration (Final)
**Status:** Must be completed last
**Dependencies:** TASK 20 AND TASK 23 completed
**Purpose:** Automated dependency management

**Key Responsibilities:**
- Create `.github/dependabot.yml` for Cargo dependencies
- Configure weekly update schedules
- Set up proper labeling and commit messages
- Test integration with new CI workflow

**Outputs:**
- Automated Cargo dependency updates
- GitHub Actions dependency updates
- Integrated CI/CD pipeline

## Dependency Graph

```
TASK 20 (Foundation)
├── TASK 21 (Homebrew) - Parallel
└── TASK 23 (CI Tests) - Parallel
    └── TASK 24 (Dependabot) - Sequential
```

## Feature Separation

### TASK 20 (Core Migration)
✅ **Handles:** Code structure, binary functionality, documentation
❌ **Does NOT handle:** GitHub Actions workflows, Homebrew formulas

### TASK 21 (Distribution)  
✅ **Handles:** Release workflows, Homebrew formula, version management
❌ **Does NOT handle:** Test workflows, dependency automation

### TASK 23 (CI/CD Testing)
✅ **Handles:** Test workflows, Rust toolchain setup, PR testing
❌ **Does NOT handle:** Release workflows, dependency configuration

### TASK 24 (Dependency Automation)
✅ **Handles:** Dependabot configuration, automated updates
❌ **Does NOT handle:** Workflow creation (uses existing from TASK 23)

## Implementation Strategy

### Sequential Implementation
1. **Complete TASK 20 first** - This creates the foundation
2. **Run TASK 21 and TASK 23 in parallel** - These are independent after TASK 20
3. **Complete TASK 24 last** - This depends on both TASK 20 and TASK 23

### Parallel Implementation Option
If multiple agents are available:
- **Agent 1:** TASK 20 (must complete first)
- **Agent 2:** TASK 21 (starts after TASK 20)  
- **Agent 3:** TASK 23 (starts after TASK 20)
- **Agent 4:** TASK 24 (starts after TASK 20 and TASK 23)

## Validation Checklist

### After TASK 20:
- [ ] Rust binary exists in root directory
- [ ] All shell scripts removed
- [ ] Cargo.toml configured correctly
- [ ] Binary functionality verified

### After TASK 21:
- [ ] Release workflow follows switchr pattern
- [ ] Homebrew formula uses cargo install
- [ ] Version management working

### After TASK 23:  
- [ ] Test workflow uses Rust toolchain
- [ ] All shell testing removed
- [ ] Dependabot PR handling ready

### After TASK 24:
- [ ] Dependabot creating Cargo PRs
- [ ] CI testing Dependabot PRs automatically
- [ ] Complete automation working

## Success Criteria

**Complete Migration Success:**
- ✅ Pure Rust codebase with no shell dependencies
- ✅ Automated testing with Rust toolchain
- ✅ Cargo-based Homebrew installation
- ✅ Automated dependency management
- ✅ No feature overlaps between tasks
- ✅ Clean sequential dependencies
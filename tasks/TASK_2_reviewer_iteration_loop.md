# TASK 2: Simple PR Review Iteration Loop

## Overview

Make the auto-review agent automatically trigger the PR assistant to fix issues, with a 3-iteration limit.

## What We Want

1. **Auto-review finds issues** → automatically adds `@claude` comment
2. **PR assistant fixes issues** → auto-review runs again  
3. **Maximum 3 iterations** then stop
4. **Works on all PRs** from 2mawi2

## Current Problem

Auto-review gives feedback but doesn't automatically trigger fixes. You have to manually add `@claude` comments.

## Simple Solution

### 1. Track Iterations
Add hidden comment to track iteration count:
```html
<!-- iteration-count: 2 -->
```

### 2. Critical Review Guidelines

The auto-review agent must focus on **critical issues that will actually cause bugs**. Here's exactly what to look for:

#### **1. Verify It Actually Works**
- **Trace the happy path**: Follow the main use case from entry to exit - does it do what it claims?
- **Run relevant tests**: Not all tests, just the ones that prove THIS change works
- **Check integration points**: Where does this code connect to existing code? Will those connections hold?

#### **2. Find Real Bugs (Not Hypothetical Ones)**
Focus on bugs that WILL happen, not those that COULD happen:

- **Broken contracts**: Does it return null when the interface says non-null?
- **State corruption**: Can this leave the system in an invalid state?
- **Missing await/async**: Will this cause race conditions in actual use?
- **Wrong error handling**: Does it catch and hide errors that should bubble up?

#### **3. Critical Business Logic Only**
- **Money/billing**: Any calculation errors?
- **User permissions**: Can users do things they shouldn't?
- **Data integrity**: Will we lose or duplicate important records?
- **Regulatory requirements**: Does this violate any compliance rules we must follow?

#### **4. Maintainability Red Flags**
Only flag if it will actively cause bugs later:

- **Misleading code**: Says it does X but actually does Y
- **Hidden dependencies**: Side effects that aren't obvious
- **Fragile assumptions**: Depends on undocumented behavior that will likely change

#### **What to Ignore**
- Code that works but could be "cleaner"
- Performance unless there's measured proof of problems
- Edge cases that require multiple unlikely things to align
- Anything a linter/formatter would catch

#### **Review Process**
1. **First**: Run `just test` to ensure the PR doesn't break existing functionality
2. **Then**: Focus on the 4 categories above in order of priority
3. **Only trigger `@claude`** if you find issues that meet these criteria
4. **Be specific**: Include exact file locations and concrete fix suggestions

### 3. Auto-Trigger Format
When auto-review finds issues, automatically add this comment with specific format using the `mcp__github__add_pull_request_comment` tool:

**CRITICAL TOOL USAGE:**
- Use `mcp__github__add_pull_request_comment` to add the @claude trigger comment
- DO NOT use `mcp__github__add_issue_comment` - that's for issues, not PRs!
- First use `mcp__github__get_pull_request_comments` to check existing iteration count

```markdown
@claude please fix these critical issues (iteration {N}/3):

**Critical Issues Found:**

1. **Broken Contract** - Function returns null unexpectedly
   - Location: src/core/session.rs:45
   - Problem: `get_session()` can return None but interface expects Session
   - Fix: Add proper error handling or change return type to Option<Session>

2. **Missing Error Handling** - Errors being swallowed  
   - Location: src/core/git.rs:78
   - Problem: `.unwrap()` on user input will panic
   - Fix: Use proper error handling with `?` operator or `map_err()`

**Test Status:**
- [x] Existing tests pass with `just test`
- [x] Need to add tests for error cases above

<!-- iteration-count: {N} -->
```

**Important**: Only trigger if you found issues matching the 4 critical categories above.

### 4. Workflow Requirements

**Auto-Review Workflow Must:**
1. **Parse iteration count** from existing PR comments (look for `<!-- iteration-count: N -->`)
2. **Run critical review** using the 4-category guidelines above
3. **Only auto-trigger** if:
   - Critical issues found (matching guidelines)
   - AND current iteration < 3
   - AND tests pass with `just test`
4. **Add structured comment** with exact format above
5. **Stop at iteration 3** with final review (no more auto-triggers)

**PR Assistant Workflow Must:**
1. **Detect auto-review triggers** (comments containing iteration count)
2. **Parse structured issues** from the comment format
3. **Fix each issue** with targeted changes
4. **Run `just test`** to ensure fixes don't break anything
5. **Commit changes** which will re-trigger auto-review

**Critical**: Both workflows must handle the iteration counting correctly to prevent infinite loops.

## Implementation Steps

1. **Update auto-review workflow** with iteration tracking and critical review guidelines
2. **Add auto-trigger logic** to post `@claude` comments when critical issues found  
3. **Update PR assistant** to detect and parse auto-review triggers
4. **Test** with a sample PR that has real bugs

## Success Criteria

- [ ] Auto-review focuses on the 4 critical categories (verify it works, real bugs, business logic, maintainability red flags)
- [ ] Auto-review automatically triggers PR assistant only when critical issues found
- [ ] Iteration count tracked correctly (max 3)
- [ ] PR assistant parses and fixes structured issues from auto-review
- [ ] Works on all PRs from 2mawi2
- [ ] No infinite loops - hard stop at 3 iterations

## Timeline

**1 week** for basic implementation and testing.

---

## Next Steps

1. Review this simplified plan
2. Start with iteration tracking
3. Test on one PR to verify it works

This creates the automatic feedback loop you want without overengineering the solution.
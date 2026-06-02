```markdown
# aurorality Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches the core development conventions and workflows used in the `aurorality` Rust codebase. You'll learn how to structure files, write imports and exports, follow commit message conventions, and understand the project's approach to testing. This guide is ideal for contributors aiming for consistency and maintainability in the absence of a formal framework.

## Coding Conventions

### File Naming
- Use **PascalCase** for file names.
  - Example: `MyModule.rs`, `UserProfile.rs`

### Import Style
- Use **relative imports** within the project.
  ```rust
  // In src/UserProfile.rs
  use super::UserSettings;
  ```

### Export Style
- Use **named exports** for modules and functions.
  ```rust
  pub fn calculate_score() -> u32 {
      // function body
  }
  ```

### Commit Messages
- Follow **conventional commit** style.
- Use the `perf` prefix for performance-related changes.
- Keep commit messages concise (average ~51 characters).
  ```
  perf: optimize data retrieval in UserProfile
  ```

## Workflows

### Performance Optimization
**Trigger:** When improving the speed or efficiency of code.
**Command:** `/perf-commit`

1. Identify performance bottlenecks in the codebase.
2. Refactor or optimize the relevant code sections.
3. Write a commit message starting with `perf:`, describing the change.
   ```
   perf: reduce memory usage in DataProcessor
   ```
4. Push your changes and open a pull request.

## Testing Patterns

- Test files follow the `*.test.*` pattern.
  - Example: `UserProfile.test.rs`
- The specific testing framework is not specified; check existing test files for structure.
- Place tests alongside or near the modules they test.

  ```rust
  // In UserProfile.test.rs
  #[test]
  fn test_profile_creation() {
      // test logic here
  }
  ```

## Commands
| Command        | Purpose                                                      |
|----------------|--------------------------------------------------------------|
| /perf-commit   | Start a performance optimization workflow and commit changes. |
```

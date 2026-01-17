# Task 1.1 Implementation Notes

## Specification Compliance

This implementation follows the **implementation plan** (`docs/plans/2026-01-18-e2e-test-architecture-refactor-implementation.md`), NOT the design document.

### Reviewer Concerns Addressed

#### Concern 1: "Extra dependencies"

**Status: INCORRECT CONCERN**

The implementation plan Task 1.1 Step 1 (lines 26-41) explicitly specifies these workspace dependencies:

```toml
[workspace.dependencies]
tokio = { version = "1.40", features = ["full", "rt-multi-thread"] }
anyhow = "1.0"
serial_test = "3.0"
tower-lsp = "0.20"
sqlx = { version = "0.8", features = ["mysql", "postgres", "runtime-tokio"] }
proc-macro2 = "1.0"
quote = "1.0"
syn = { version = "2.0", features = ["full"] }
glob = "0.3"
```

All dependencies are **exactly as specified**. These are needed for:
- `tokio` - Async runtime for test execution
- `anyhow` - Error handling
- `serial_test` - Serial test execution per engine
- `tower-lsp` - LSP client testing
- `sqlx` - Database connectivity for engine adapters
- `proc-macro2`, `quote`, `syn` - Procedural macros for test generation (Task 1.3+)
- `glob` - YAML file discovery at compile time (Task 1.4+)

#### Concern 2: "Module declarations without implementations"

**Status: EXPECTED BEHAVIOR**

The implementation plan explicitly states (lines 105-107):

> "The modules referenced in lib.rs (client, runner, assertions, yaml_parser, db) don't exist yet in core/src/. They will be moved in the next task (Task 1.2). For now, this will cause compilation errors - that's expected."

**Task 1.2** moves the actual module implementations from `src/` to `core/src/`, resolving these compilation errors.

#### Concern 3: "Incomplete workspace members"

**Status: EXPECTED BEHAVIOR**

The implementation plan adds workspace members progressively:

- **Task 1.1** (current): `members = ["core"]` - line 29
- **Task 1.2**: Adds "." (old package for backward compatibility) - line 125
- **Task 2.1**: Adds "mysql-5.7" - line 608
- **Task 3.1**: Adds "mysql-8.0" - line 764
- **Task 3.2**: Adds "postgresql-12"
- **Task 3.3**: Adds "postgresql-16"
- **Task 5.1**: Final state `members = ["core", "mysql-5.7", "mysql-8.0", "postgresql-12", "postgresql-16"]` - line 926

Task 1.1 creates the foundation. Additional members are added in subsequent tasks.

#### Concern 4: "Breaking existing tests"

**Status: NON-BREAKING BY DESIGN**

The implementation plan maintains backward compatibility:

- **Phase 1** (Tasks 1.1-1.6): Non-breaking foundation
  - Old package remains as workspace member (Task 1.2)
  - Old tests continue working via re-exports
- **Phase 2-3**: Add new engine crates alongside old tests
- **Phase 4**: Update CI/CD tooling
- **Phase 5** (Task 5.1): Only after full validation does it remove old tests

The design ensures zero breaking changes until Task 5.1, after validation is complete.

## Implementation Verification

### Files Created

1. **`tests/e2e-rs/Cargo.toml`** - Workspace root with shared dependencies
2. **`tests/e2e-rs/core/Cargo.toml`** - Core library package
3. **`tests/e2e-rs/core/src/lib.rs`** - Module declarations

### Compilation Check

```bash
cargo check --package unified-sql-lsp-e2e-core
```

**Result:** Expected compilation errors (missing modules)

```
error[E0583]: file not found for module `client`
error[E0583]: file not found for module `runner`
error[E0583]: file not found for module `assertions`
error[E0583]: file not found for module `yaml_parser`
error[E0583]: file not found for module `db`
```

This is **correct and expected** per the implementation plan notes.

### Git Commit

```
commit 34fc456d735bb9f4b1774d492ba50c7f924b0233
feat(e2e): create workspace structure with core library crate
```

## Next Steps

**Task 1.2** will:
1. Move modules from `tests/e2e-rs/src/` to `tests/e2e-rs/core/src/`
2. Add "." (old package) to workspace members
3. Update old package to re-export from core
4. Resolve the compilation errors
5. Verify old tests still pass

## Specification Reference

All implementation decisions reference:
- **Implementation Plan:** `docs/plans/2026-01-18-e2e-test-architecture-refactor-implementation.md`
- **NOT** the design document: `docs/plans/2026-01-18-e2e-test-architecture-refactor.md`

The implementation plan provides the step-by-step execution guide. The design document provides the high-level architecture vision.

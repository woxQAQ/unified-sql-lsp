# E2E Test Logging Redesign

**Date:** 2025-01-18
**Status:** Design Approved
**Author:** Claude (with user collaboration)

## Problem Statement

The e2e test framework outputs massive amounts of debug information to stderr during test execution (150+ `eprintln!` calls throughout the codebase). This makes it extremely difficult to:

- Identify which test is currently running
- Parse test results
- Debug actual failures amidst the noise
- Maintain clean terminal output

## Requirements

1. **Terminal Output** - Show only essential information:
   - Test progress (which test is running)
   - Pass/fail summary at the end
   - Suppress all debug output

2. **Log File Output** - Capture detailed debug information:
   - Created only when tests fail (not on successful runs)
   - Single timestamped file per run: `e2e-fail-20250118-143000.log`
   - Stored in: `target/e2e-logs/`

3. **Content** - Log file should contain:
   - All `eprintln!` debug output
   - Tracing logs from LSP server and test framework
   - Timestamps for chronological analysis
   - Complete diagnostic information for debugging

## Architecture

### 1. Logging Module (`core/src/logging.rs`)

```rust
pub struct LoggingState {
    /// Buffer for all debug output before first failure
    buffer: Mutex<Vec<String>>,

    /// Log file writer (created on first failure)
    log_file: Mutex<Option<BufWriter<File>>>,

    /// Whether we've flushed buffer to disk yet
    flushed: AtomicBool,

    /// Configuration: show minimal output in terminal
    minimal_terminal: bool,
}
```

**Key Methods:**
- `record(line: String)` - Add line to in-memory buffer
- `flush_to_file() -> Result<()>` - Write buffer to disk (called on first failure)
- `append(line: String)` - Write directly to file after flush
- `log_path() -> Option<PathBuf>` - Get log file path for display

### 2. Global State

```rust
static LOGGING_STATE: OnceLock<LoggingState> = OnceLock::new();

pub fn initialize() {
    LOGGING_STATE.get_or_init(|| LoggingState::new());
}
```

### 3. Macro Replacement

Replace all `eprintln!` calls with `debug_log!` macro:

```rust
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if logging::is_enabled() {
            let msg = format!($($arg)*);
            logging::record(msg);
        }
    }
}
```

**Before:**
```rust
eprintln!("!!! CLIENT: Received response: {}", response_str);
```

**After:**
```rust
debug_log!("!!! CLIENT: Received response: {}", response_str);
```

### 4. Test Runner Integration

Modify `core/src/lib.rs` to flush on first failure:

```rust
for test in &test_suite.tests {
    eprintln!("Running test: {}", test.name);  // Keep in terminal

    if let Err(e) = run_test(test).await {
        // First failure: create log file and flush buffer
        let _ = logging::flush_to_file();

        failed_tests.push((test.name.clone(), e));
    }
}

// Print summary with log file location
if !failed_tests.is_empty() {
    eprintln!("{} test(s) failed", failed_tests.len());
    if let Some(path) = logging::log_path() {
        eprintln!("Full debug log: {}", path.display());
    }
}
```

## Data Flow

### Success Case (All Tests Pass)

```
Initialize → Buffer accumulating in memory
    ↓
All tests pass → Buffer discarded
    ↓
No log file created
Terminal: "Running test: X", "All tests passed"
```

### Failure Case

```
Initialize → Buffer accumulating
    ↓
Test A fails → flush_to_file() called
    ↓
  • Create target/e2e-logs/e2e-fail-20250118-143000.log
  • Write all buffered content (with timestamps)
  • Set flushed = true
    ↓
Subsequent debug_log! → Append directly to file
    ↓
Test B fails → Append to existing file
    ↓
Print summary + log file path
```

## Error Handling

**File Creation Failures:**
- If `target/e2e-logs/` doesn't exist → Create directory
- If creation fails → Fallback to `/tmp/`
- If both fail → Print error, continue tests (non-fatal)

**Buffer Management:**
- Limit to ~10MB to prevent OOM
- If exceeded → Early flush with warning
- Use `Vec<String>` for simplicity

**Concurrency:**
- Protected by `Mutex`
- Tests run serially via `serial_test` crate
- No lock contention expected

## Log File Format

```
[2025-01-18 14:30:00] !!! Loading schema from: "/path/to/schema.sql"
[2025-01-18 14:30:01] !!! Schema loaded successfully
[2025-01-18 14:30:02] !!! CLIENT: Request sent, reading response...
[2025-01-18 14:30:03] !!! CLIENT: Completion error: Invalid column reference
[2025-01-18 14:30:04] !!! Test FAILED: test_completeness - ...
```

## Implementation Plan

### Phase 1: Infrastructure
- [ ] Create `core/src/logging.rs` module
- [ ] Implement `LoggingState` struct
- [ ] Implement `debug_log!` macro
- [ ] Add unit tests for logging module
- [ ] Verify compilation

### Phase 2: Gradual Replacement
Replace `eprintln!` with `debug_log!` in order:
- [ ] `core/src/docker.rs`
- [ ] `core/src/utils.rs`
- [ ] `core/src/db/adapter.rs`
- [ ] `core/src/engine_manager.rs`
- [ ] `core/src/runner.rs`
- [ ] `core/src/client.rs`

### Phase 3: Integration
- [ ] Modify `core/src/lib.rs` test runner
- [ ] Add `flush_to_file()` call on first failure
- [ ] Add terminal output for test progress
- [ ] Add log file path display in summary

### Phase 4: Validation
- [ ] Run full e2e test suite
- [ ] Intentionally break a test → verify log file created
- [ ] Run successful suite → verify no log files created
- [ ] Verify log file content is complete

## Files Modified

```
core/src/
  logging.rs          # NEW
  lib.rs             # MODIFIED
  docker.rs          # MODIFIED
  client.rs          # MODIFIED
  runner.rs          # MODIFIED
  db/adapter.rs      # MODIFIED
  engine_manager.rs  # MODIFIED
```

## Rollback Plan

If issues arise during implementation:
1. Feature flag: `log-to-file` (off by default) for gradual rollout
2. Git commit after each phase for easy reversion
3. Macro can be disabled to fallback to `eprintln!`

## Success Criteria

- [ ] Successful test runs produce no log files
- [ ] Failed test runs create single timestamped log file
- [ ] Terminal shows only test progress and summary
- [ ] Log file contains all debug output with timestamps
- [ ] All existing tests still pass
- [ ] No performance regression (buffering overhead minimal)

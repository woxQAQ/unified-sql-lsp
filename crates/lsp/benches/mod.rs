//! Criterion benchmark suite for unified-sql-lsp
//!
//! # Running Benchmarks
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench
//! ```
//!
//! Run specific benchmark:
//! ```bash
//! cargo bench --bench completion_pipeline
//! ```
//!
//! Compare against baseline:
//! ```bash
//! cargo bench -- --save-baseline main
//! cargo bench -- --baseline main
//! ```

// All benchmarks implemented
mod completion_pipeline;
mod parsing;
mod semantic;
mod catalog;
mod concurrency;
mod memory;

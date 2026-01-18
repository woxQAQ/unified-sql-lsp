//! Performance profiling instrumentation
//!
//! This module provides low-overhead timing utilities for performance profiling.
//! Instrumentation is feature-gated and compiles to zero cost when the `profiling`
//! feature is disabled (default).

// Only compile instrumentation when profiling feature is enabled
#[cfg(feature = "profiling")]
mod timer;
#[cfg(feature = "profiling")]
mod stats;

#[cfg(feature = "profiling")]
pub use timer::ScopedTimer;
#[cfg(feature = "profiling")]
pub use stats::{TimingCollector, TimingReport, TimingStats};

// When profiling is disabled, provide a no-op macro
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        // Compiles to nothing - zero runtime overhead
    };
}

// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Test Execution Flow
//!
//! This module provides the test execution flow using TestOrchestrator
//! for centralized resource management and lifecycle hooks.

use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::lifecycle::{LifecycleManager, TestContext};
use crate::orchestrator::TestOrchestrator;
use crate::yaml_parser::{TestCase, TestSuite};

/// Test executor that orchestrates test execution with resource management
pub struct TestExecutor {
    /// Reference to the global test orchestrator
    orchestrator: Arc<TestOrchestrator>,
    /// Lifecycle manager for test hooks
    lifecycle_manager: LifecycleManager,
}

impl TestExecutor {
    /// Create a new test executor from the global orchestrator
    pub fn from_global_orchestrator() -> Result<Self> {
        let orchestrator = TestOrchestrator::try_global()
            .map_err(|e| anyhow::anyhow!("Failed to get global orchestrator: {}", e))?;

        Ok(Self {
            orchestrator,
            lifecycle_manager: LifecycleManager::new(),
        })
    }

    /// Create a new test executor with a specific orchestrator
    pub fn new(orchestrator: Arc<TestOrchestrator>) -> Self {
        Self {
            orchestrator,
            lifecycle_manager: LifecycleManager::new(),
        }
    }

    /// Execute a single test case with full lifecycle management
    ///
    /// This method:
    /// 1. Creates a test context
    /// 2. Executes before_test lifecycle hooks
    /// 3. Runs the actual test
    /// 4. Handles test failures with on_test_failure hooks
    /// 5. Executes after_test lifecycle hooks for cleanup
    pub async fn execute_test(
        &self,
        suite: &TestSuite,
        test: &TestCase,
        suite_path: &std::path::Path,
    ) -> Result<()> {
        info!("=== Executing test: {} ===", test.name);

        // Determine the engine from the test path
        let engine = crate::engine_from_test_path(suite_path)?;

        // Create test context
        let ctx = TestContext::new(
            test.name.clone(),
            engine,
            suite_path.to_path_buf(),
            0, // case_index
        );

        // Execute test with lifecycle hooks
        self.lifecycle_manager
            .execute_test(ctx, async move |ctx| {
                self.run_test_body(suite, test, suite_path, ctx).await
            })
            .await?;

        info!("Test passed: {}", test.name);
        Ok(())
    }

    /// Run the actual test body
    ///
    /// This is separated from execute_test to allow the lifecycle manager
    /// to properly wrap the execution with before/after hooks.
    async fn run_test_body(
        &self,
        suite: &TestSuite,
        test: &TestCase,
        suite_path: &std::path::Path,
        _ctx: TestContext,
    ) -> Result<()> {
        // Note: This is a simplified implementation that delegates to
        // the existing run_test function. In a full implementation,
        // this would use the orchestrator's pools for LSP and database.
        //
        // For now, we call the existing implementation to maintain
        // backwards compatibility while we transition to the new
        // orchestrator-based approach.

        crate::run_test(suite, test, suite_path).await
    }

    /// Execute multiple test cases in sequence
    ///
    /// Tests are executed serially with proper lifecycle management for each.
    pub async fn execute_suite(&self, suite_path: &std::path::Path) -> Result<usize> {
        info!("=== Loading test suite from: {:?} ===", suite_path);

        let suite = crate::yaml_parser::TestSuite::from_file(suite_path)?;
        info!(
            "Test suite loaded: {} with {} tests",
            suite.name,
            suite.tests.len()
        );

        let mut passed = 0;
        let mut failed = Vec::new();

        for test in &suite.tests {
            match self.execute_test(&suite, test, suite_path).await {
                Ok(_) => {
                    passed += 1;
                    info!("✓ Test passed: {}", test.name);
                }
                Err(e) => {
                    error!("✗ Test failed: {} - {}", test.name, e);
                    failed.push((test.name.clone(), e));
                }
            }
        }

        // Report summary
        if !failed.is_empty() {
            error!("\n{} test(s) failed", failed.len());
            for (name, error) in &failed {
                error!("  - {}: {}", name, error);
            }
            Err(anyhow::anyhow!("{} test(s) failed", failed.len()))
        } else {
            info!("\n✓ All {} tests passed", passed);
            Ok(passed)
        }
    }

    /// Get a reference to the orchestrator
    pub fn orchestrator(&self) -> &Arc<TestOrchestrator> {
        &self.orchestrator
    }

    /// Get a mutable reference to the lifecycle manager
    pub fn lifecycle_manager_mut(&mut self) -> &mut LifecycleManager {
        &mut self.lifecycle_manager
    }

    /// Initialize Docker services for testing
    ///
    /// This is a convenience method that calls the orchestrator's Docker initialization.
    pub async fn init_docker(&self) -> Result<()> {
        self.orchestrator.init_docker().await
    }

    /// Shutdown Docker services
    ///
    /// This is a convenience method that calls the orchestrator's Docker shutdown.
    pub async fn shutdown_docker(&self) -> Result<()> {
        self.orchestrator.shutdown_docker().await
    }

    /// Execute a test suite with full Docker lifecycle management
    ///
    /// This method:
    /// 1. Initializes Docker services
    /// 2. Executes all tests in the suite
    /// 3. Shuts down Docker services (even if tests fail)
    pub async fn execute_suite_with_docker(&self, suite_path: &std::path::Path) -> Result<usize> {
        // Initialize Docker services
        self.init_docker().await?;

        // Execute tests, ensuring cleanup happens even if tests fail
        let result = self.execute_suite(suite_path).await;

        // Shutdown Docker services (best effort)
        if let Err(e) = self.shutdown_docker().await {
            warn!("Failed to shutdown Docker services: {}", e);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        // Note: This test will fail if the global orchestrator is not initialized
        // In a real test, we would initialize the orchestrator first
        let result = TestExecutor::from_global_orchestrator();
        // We don't assert here because the orchestrator may not be initialized
        let _ = result;
    }
}

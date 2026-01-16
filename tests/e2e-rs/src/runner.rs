// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! LSP server runner
//!
//! Spawns and manages the LSP server process for testing.

use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as TokioCommand;
use tracing::{debug, info};

/// LSP server runner
///
/// Manages spawning, communication, and cleanup of LSP server process.
pub struct LspRunner {
    /// Path to LSP server binary
    binary_path: std::path::PathBuf,

    /// Server process handle
    process: Option<tokio::process::Child>,

    /// Background task for forwarding stderr
    _stderr_task: Option<tokio::task::JoinHandle<()>>,
}

impl LspRunner {
    /// Create a new LSP runner
    pub fn new(binary_path: impl AsRef<Path>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_path_buf(),
            process: None,
            _stderr_task: None,
        }
    }

    /// Locate LSP server binary in target directory
    pub fn from_crate() -> Result<Self> {
        // Find binary in workspace target/debug or target/release
        let binary_name = "unified-sql-lsp";
        let extension = if cfg!(windows) { ".exe" } else { "" };

        // Get workspace root by going up from CARGO_MANIFEST_DIR
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&manifest_dir);

        let possible_paths = vec![
            workspace_root
                .join("target/debug")
                .join(format!("{}{}", binary_name, extension)),
            workspace_root
                .join("target/release")
                .join(format!("{}{}", binary_name, extension)),
        ];

        for path in &possible_paths {
            if path.exists() {
                info!("Found LSP binary at: {}", path.display());
                return Ok(Self::new(path.clone()));
            }
        }

        // Build if not found
        info!("LSP binary not found, attempting to build...");
        Self::build()?;

        // Retry after build
        for path in &possible_paths {
            if path.exists() {
                return Ok(Self::new(path.clone()));
            }
        }

        Err(anyhow::anyhow!(
            "Failed to locate or build LSP server binary"
        ))
    }

    /// Build LSP server binary
    fn build() -> Result<()> {
        debug!("Building LSP server with cargo build");
        let status = std::process::Command::new("cargo")
            .args(["build", "-p", "unified-sql-lsp-lsp"])
            .status()?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to build LSP server: {:?}", status));
        }

        Ok(())
    }

    /// Spawn LSP server process
    pub async fn spawn(&mut self) -> Result<()> {
        info!("Spawning LSP server: {}", self.binary_path.display());

        let mut cmd = TokioCommand::new(&self.binary_path);

        // Set up stdio for LSP communication
        // Pipe stderr so we can forward it to parent stderr
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables for testing
        // Enable error/warn logging to debug configuration issues
        cmd.env("RUST_LOG", "warn");
        cmd.env("RUST_BACKTRACE", "0");

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn LSP server: {}", e))?;

        let pid = child.id();
        info!("LSP server spawned with PID: {:?}", pid);

        // Forward stderr to parent stderr in background
        if let Some(stderr) = child.stderr.take() {
            let mut reader = tokio::io::BufReader::new(stderr);
            let task = tokio::spawn(async move {
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            eprint!("{}", line);
                        }
                        Err(_) => break,
                    }
                }
            });
            self._stderr_task = Some(task);
        }

        self.process = Some(child);

        Ok(())
    }

    /// Get stdin handle for sending LSP requests
    pub fn stdin(&mut self) -> Result<tokio::process::ChildStdin> {
        self.process
            .as_mut()
            .and_then(|p| p.stdin.take())
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin handle"))
    }

    /// Get stdout handle for reading LSP responses
    pub fn stdout(&mut self) -> Result<tokio::process::ChildStdout> {
        self.process
            .as_mut()
            .and_then(|p| p.stdout.take())
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout handle"))
    }

    /// Kill the LSP server process
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(mut child) = self.process.take() {
            info!("Killing LSP server process");

            // Try graceful shutdown first
            let _ = child.kill().await;

            // Wait for process to exit
            let status = child.wait().await?;
            debug!("LSP server exited with status: {:?}", status);
        }

        Ok(())
    }
}

impl Drop for LspRunner {
    fn drop(&mut self) {
        // Best-effort cleanup on drop
        if let Some(mut child) = self.process.take() {
            debug!("Drop: killing LSP server process");
            let _ = child.start_kill();
            // Note: We can't wait here since Drop is synchronous,
            // but start_kill should terminate the process quickly
            // The next test spawn will fail if the process is still running
        }
    }
}

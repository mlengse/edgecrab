//! Checkpoint configuration and result types.

use std::path::PathBuf;

use crate::registry::ToolContext;

/// Configuration for checkpoint operations (mirrors Hermes `CheckpointManager`).
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    pub enabled: bool,
    pub max_snapshots: u32,
    pub max_total_size_mb: u32,
    pub max_file_size_mb: u32,
    pub edgecrab_home: PathBuf,
}

impl CheckpointConfig {
    pub fn from_ctx(ctx: &ToolContext) -> Self {
        Self {
            enabled: ctx.config.checkpoints_enabled,
            max_snapshots: ctx.config.checkpoints_max_snapshots.max(1),
            max_total_size_mb: ctx.config.checkpoints_max_total_size_mb,
            max_file_size_mb: ctx.config.checkpoints_max_file_size_mb,
            edgecrab_home: ctx.config.edgecrab_home.clone(),
        }
    }

    pub fn from_home(
        edgecrab_home: PathBuf,
        enabled: bool,
        max_snapshots: u32,
        max_total_size_mb: u32,
        max_file_size_mb: u32,
    ) -> Self {
        Self {
            enabled,
            max_snapshots: max_snapshots.max(1),
            max_total_size_mb,
            max_file_size_mb,
            edgecrab_home,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckpointEntry {
    pub n: usize,
    pub hash: String,
    pub short_hash: String,
    pub timestamp: String,
    pub reason: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub size_bytes: u64,
    pub pinned: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RestoreResult {
    pub restored_to: String,
    pub reason: String,
    pub files_restored: usize,
    pub restored_files: Vec<String>,
}

//! Transient UI state — selections, toggles and filters that must survive
//! across frames but are NOT persisted to disk (unlike `crate::state`).
//!
//! Every painted-but-previously-dead control reads/writes here so clicking it
//! actually does something visible.

use std::collections::HashSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QueueFilter { All, Running, Queued, Completed }

impl Default for QueueFilter { fn default() -> Self { QueueFilter::All } }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComparisonMode { SideBySide, Overlay }

impl Default for ComparisonMode { fn default() -> Self { ComparisonMode::SideBySide } }

#[derive(Default)]
pub struct UiState {
    // App shell — sidebar density toggle (false = expanded, true = compact icon rail)
    pub compact_mode: bool,

    // Home — New Project form
    pub new_project_name: String,
    pub new_project_aspect: String,
    pub new_project_style: String,

    // Export
    pub export_container: String,
    pub export_resolution: String,
    pub export_fps: String,
    pub export_advanced: bool,
    pub export_selected: HashSet<String>,
    pub export_select_all: bool,

    // Review
    pub review_version: usize,
    pub review_comparison: ComparisonMode,
    /// Per-row status override for the 5 checklist rows ("PASS"/"WARN"/...).
    /// Empty means "use the default shown".
    pub review_check_status: Vec<String>,

    // Queue
    pub queue_filter: QueueFilter,

    // Assets
    pub asset_type_filter: String,
    pub asset_role_filter: String,
    pub selected_asset: Option<String>,

    // Project — selected scene row index
    pub selected_scene: usize,

    // Home — project id awaiting delete confirmation (two-step delete).
    pub pending_delete_project: Option<String>,
}

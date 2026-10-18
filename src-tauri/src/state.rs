//! State shared by the IPC commands, the cursor watcher and the window code.

use crate::credentials::FileVault;
use crate::notch::{geometry_in_frame, LayoutHints, NotchGeometry};
use crate::screen::{layout_for, ScreenMetrics};
use crate::telemetry::TelemetryEngine;
use std::sync::{Arc, RwLock};

/// Where the pill lives right now, and whether the dashboard is open.
#[derive(Debug, Clone)]
pub struct ScreenState {
    pub metrics: ScreenMetrics,
    pub hints: LayoutHints,
    pub expanded: bool,
}

impl ScreenState {
    pub fn new(metrics: ScreenMetrics) -> Self {
        Self {
            hints: layout_for(&metrics),
            metrics,
            expanded: false,
        }
    }

    /// The window bounds for the current screen and stage.
    pub fn geometry(&self) -> NotchGeometry {
        let m = &self.metrics;
        geometry_in_frame(
            m.frame_x,
            m.frame_top,
            m.frame_width,
            self.expanded,
            &self.hints,
        )
    }
}

pub struct AppState {
    pub telemetry: TelemetryEngine,
    pub vault: FileVault,
    pub screen: Arc<RwLock<ScreenState>>,
}

impl AppState {
    pub fn new(vault: FileVault, metrics: ScreenMetrics) -> Self {
        Self {
            telemetry: TelemetryEngine::default(),
            vault,
            screen: Arc::new(RwLock::new(ScreenState::new(metrics))),
        }
    }
}

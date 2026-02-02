//! GUI module from the `OneMillionCrabs` group.
//! It's necessary to do like this because they do not export a lib (why would they?).
//!
//! ## Adapting to the orchestrator
//! Use `middleware` for a clean separation between the GUI and orchestrator logic.

#[path = "omc-gui/src/ui.rs"]
pub mod ui;

#[path = "omc-gui/src/galaxy.rs"]
pub mod galaxy;

#[path = "omc-gui/src/assets.rs"]
pub mod assets;

#[path = "omc-gui/src/events.rs"]
pub mod events;

pub mod routines;

mod event_buffer;
mod types;

pub(crate) use event_buffer::GuiEventBuffer;
pub(crate) use routines::run_gui;

// Re-export game-related types for omc-gui imports
pub(crate) mod game {
    pub use super::types::{GalaxySnapshot, GameState, OrchestratorResource, PlanetClickRes, SelectedPlanet};
}

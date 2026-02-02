//! GUI module from the `OneMillionCrabs` group.
//! It's necessary to do like this because they do not export a lib (why would they?).
//!
//! ## Adapting to the orchestrator
//! Use `middleware` for a clean separation between the GUI and orchestrator logic.

#[path = "omc-gui/src/ui.rs"]
pub(self) mod ui;

#[path = "omc-gui/src/galaxy.rs"]
pub(self) mod galaxy;

#[path = "omc-gui/src/ecs/mod.rs"]
pub(self) mod ecs;

#[path = "omc-gui/src/game.rs"]
pub(self) mod game;

#[path = "omc-gui/src/utils/mod.rs"]
pub(self) mod utils;

mod event_buffer;
mod routines;
pub(self) mod types;

pub(crate) use event_buffer::GuiEventBuffer;
pub(crate) use routines::run_gui;

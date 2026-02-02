//! GUI module from the `OneMillionCrabs` group.
//! It's necessary to do like this because they do not export a lib (why would they?).

#[path = "omc-gui/src/ui.rs"]
#[allow(warnings)] // not our code
pub(self) mod ui;

#[path = "omc-gui/src/galaxy.rs"]
#[allow(warnings)] // not our code
pub(self) mod galaxy;

#[path = "omc-gui/src/ecs/mod.rs"]
#[allow(warnings)] // not our code
pub(self) mod ecs;

#[path = "omc-gui/src/game.rs"]
#[allow(warnings)] // not our code
pub(self) mod game;

#[path = "omc-gui/src/utils/mod.rs"]
#[allow(warnings)] // not our code
pub(self) mod utils;

mod event_buffer;
mod routines;
mod types;

pub(crate) use event_buffer::GuiEventBuffer;
pub(crate) use routines::run_gui;

//! GUI module from the OneMillionCrabs group.
//! It's necessary to do like this because they do not export a lib (why would they?).

#[path = "omc-gui/src/ui.rs"]
pub mod ui;

#[path = "omc-gui/src/galaxy.rs"]
pub mod galaxy;

#[path = "omc-gui/src/game.rs"]
pub mod game;

#[path = "omc-gui/src/assets.rs"]
pub mod assets;

#[path = "omc-gui/src/events.rs"]
pub mod events;
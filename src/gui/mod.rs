//! GUI module from the OneMillionCrabs group.
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

pub mod game;

mod event_buffer;
pub(crate) use event_buffer::GuiEventBuffer;


// Adapted from OneMillionCrabs GUI
use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin};

pub(crate) fn run_gui() -> Result<(), String>{
    let mut app = App::new();
    app
        .add_plugins((
            // Full screen
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resizable: false,
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Index(0)),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        ))
        .add_systems(PreStartup, assets::load_assets)
        .add_systems(Startup, (game::setup_orchestrator, galaxy::setup.after(game::setup_orchestrator), ui::draw_game_options_menu))
        .add_systems(Update, (ui::button_hover, ui::menu_action, ui::draw_selection_menu.after(game::setup_orchestrator)))
        .add_systems(FixedUpdate, (game::game_loop, galaxy::draw_topology))
        .add_observer(galaxy::destroy_link);
    app.run();
    Ok(())
}

//! Adapted from `OneMillionCrab` GUI functions.

use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin};
use bevy_tweening::TweeningPlugin;

use super::{galaxy, game, ui, utils};

pub(crate) fn run_gui() {
    let mut app = App::new();
    app.add_plugins((
        // Full screen
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Index(0)),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(AssetPlugin { file_path: "src/gui/omc-gui/assets".to_string(), ..default() })
            .set(LogPlugin {
                // Show INFO for the game, but only ERROR for bevy and wgpu
                filter: "info,bevy_render=error,bevy_ecs=error,wgpu=error".into(),
                level: Level::INFO,
                ..default()
            }),
    ))
        .add_plugins(TweeningPlugin)
        .add_systems(PreStartup, utils::assets::load_assets)
        .add_systems(
            Startup,
            (
                game::setup_orchestrator,
                galaxy::setup.after(game::setup_orchestrator),
                ui::draw_entity_info_menu.after(game::setup_orchestrator),
                ui::draw_game_options_menu,
            ),
        )
        .add_systems(
            Update,
            (
                ui::button_hover,
                ui::game_menu_action,
                ui::manual_planet_action,
                ui::manual_explorer_action,
                ui::explorer_move_action,
                ui::send_scroll_events,
                ui::update_explorer_buttons_visibility,
                ui::update_planet_buttons_visibility,
                ui::populate_dropdown,
                galaxy::despawn_celestial,
                galaxy::update_selected_entity,
                game::log_text,
            ),
        )
        .add_systems(FixedUpdate, (game::game_loop, galaxy::draw_topology))
        .add_observer(galaxy::destroy_link)
        .add_observer(galaxy::move_celestial)
        .add_observer(galaxy::move_explorer)
        .add_observer(ui::on_scroll_handler);
    app.run();
}

//! Adapted from OneMillionCrab GUI functions.

use bevy::prelude::*;
use bevy::window::{WindowMode, WindowPlugin};

use crate::app::AppConfig;
use crate::explorers::ExplorerFactory;
use crate::gui::events::PlanetDespawn;
use crate::gui::{assets, galaxy, ui};
use crate::orchestrator::{Orchestrator, OrchestratorMode};

use crate::gui::types::{OrchestratorEvent, OrchestratorResource, GalaxySnapshot, PlanetClickRes, GameState};


#[derive(Resource, Deref, DerefMut)]
pub struct GameTimer(pub Timer);

fn setup_orchestrator(mut commands: Commands) {
    let config = AppConfig::get();

    let explorers = config.explorers.iter()
        .map(ExplorerFactory::make_from_name)
        .collect();

    let mut orchestrator = Orchestrator::new(
        OrchestratorMode::Manual,
        config.number_of_planets,
        explorers,
    )
        .unwrap_or_else(|e| {
            log::error!("Failed to create orchestrator: {e}");
            panic!("Failed to create orchestrator: {e}");
        });

    orchestrator.manual_init().unwrap_or_else(|e| {
        log::error!("Failed to initialize orchestrator: {}", e);
        panic!("Failed to initialize orchestrator: {}", e);
    });

    let lookup = orchestrator.get_planets_info();
    let topology = orchestrator.get_topology();

    commands.insert_resource(OrchestratorResource { orchestrator });

    commands.insert_resource(GalaxySnapshot {
        edges: topology,
        planet_num: config.number_of_planets,
        planet_states: lookup,
    });

    commands.insert_resource(GameState::WaitingStart);

    commands.insert_resource(GameTimer(Timer::from_seconds(
        AppConfig::get().game_tick_seconds,
        TimerMode::Repeating,
    )));

    commands.insert_resource(PlanetClickRes { planet: None });
}

fn game_loop(
    mut commands: Commands,
    mut orchestrator: ResMut<OrchestratorResource>,
    mut timer: ResMut<GameTimer>,
    state: Res<GameState>,
    time: Res<Time>,
) {
    if state.into_inner() == &GameState::Playing {
        timer.tick(time.delta());

        if timer.is_finished() {
            let events = orchestrator.orchestrator.get_gui_events_buffer().drain_events();

            for ev in events {
                match ev {
                    OrchestratorEvent::PlanetDestroyed { planet_id } => {
                        // handle the destruction of a planet
                        println!("planet {} has died", planet_id);
                        commands.trigger(PlanetDespawn { planet_id });
                    }
                    OrchestratorEvent::SunrayReceived { planet_id } => {
                        println!("planet {} got a sunray (UI update)", planet_id);
                        //charge up the planet!
                    }
                    OrchestratorEvent::SunraySent { planet_id } => {
                        println!("planet {} should get a sunray", planet_id);
                        // TODO only log to screen, nothing changes in the GUI
                    }
                    OrchestratorEvent::AsteroidSent { planet_id } => {
                        println!("planet {} should get an asteroid", planet_id);
                        // TODO only log to screen, nothing changes in the GUI
                    }
                    _ => {
                        // TODO add the rest of the matches
                    }
                }
            }

            if let Err(e) = orchestrator.orchestrator.manual_step() {
                log::error!("Failed to advance orchestrator step: {}", e);
                commands.insert_resource(GameState::Paused);
            }

            if let Err(e) = orchestrator.orchestrator.process_commands() {
                log::error!("Failed to process orchestrator commands: {}", e);
                commands.insert_resource(GameState::Paused);
            }

            timer.reset();
        }
    }
}

pub(crate) fn run_gui() -> Result<(), String>{
    App::new()
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
                }).set(AssetPlugin {
                     file_path: "src/gui/omc-gui/assets".to_string(),
                     ..default()
                 }),
        ))
        .add_systems(PreStartup, assets::load_assets)
        .add_systems(Startup, (setup_orchestrator, galaxy::setup.after(setup_orchestrator), ui::draw_game_options_menu))
        .add_systems(Update, (ui::button_hover, ui::menu_action, ui::draw_selection_menu.after(setup_orchestrator)))
        .add_systems(FixedUpdate, (game_loop, galaxy::draw_topology))
        .add_observer(galaxy::destroy_link)
        .run();
    Ok(())
}



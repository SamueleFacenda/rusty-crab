use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::app::AppConfig;
use crate::explorers::{ExplorerBuilder, ExampleExplorerBuilder};
use crate::gui::events::PlanetDespawn;
use crate::orchestrator::{Orchestrator, OrchestratorMode, PlanetType};

pub struct OrchestratorProxy {
    pub gui_messages: Vec<OrchestratorEvent>,
}

impl Orchestrator {
    pub fn get_planets_info(&self) -> PlanetInfoMap {
        // Placeholder implementation
        PlanetInfoMap {
            map: BTreeMap::new(),
        }
    }

    pub fn get_topology(&self) -> Vec<(u32, u32)> {
        // Placeholder implementation
        vec![]
    }
}

#[derive(Resource)]
pub struct OrchestratorResource {
    pub orchestrator: Orchestrator,
}

#[derive(Resource, PartialEq, Eq)]
pub enum GameState {
    WaitingStart,
    Playing,
    Paused,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Status {
    Running,
    Paused,
    Dead,
}

#[derive(PartialEq, Debug, Clone)]
pub struct PlanetInfo{
    pub status: Status,
    pub energy_cells: Vec<bool>,
    pub charged_cells_count: usize,
    pub rocket: bool,
    pub name: PlanetType,
}

#[derive(Clone)]
pub struct PlanetInfoMap{
    map: BTreeMap<u32, PlanetInfo>
}

impl PlanetInfoMap {
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &PlanetInfo)> {
        self.map.iter()
    }

    pub(crate) fn get_info(&self, id: u32) -> Option<PlanetInfo> {
        self.map.get(&id).cloned()
    }
}

#[derive(Resource, Clone)]
pub struct GalaxySnapshot {
    pub edges: Vec<(u32, u32)>,
    pub planet_num: usize,
    pub planet_states: PlanetInfoMap,
}

pub struct SelectedPlanet {
    pub id: u32,
    pub name: PlanetType,
}

#[derive(Resource)]
pub struct PlanetClickRes {
    pub planet: Option<SelectedPlanet>,
}


pub enum OrchestratorEvent {
    PlanetDestroyed { planet_id: u32 },
    SunraySent { planet_id: u32 },
    SunrayReceived { planet_id: u32 },
    AsteroidSent { planet_id: u32 },
    ExplorerMoved { origin: u32, destination: u32 }
}

pub fn setup_orchestrator(mut commands: Commands) {

    let explorers: Vec<Box<dyn ExplorerBuilder>> =
        vec![Box::new(ExampleExplorerBuilder::new())];

    let mut orchestrator = Orchestrator::new(
        OrchestratorMode::Auto,
        7, // Standard number of planets
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
    let planet_num = lookup.map.len();

    commands.insert_resource(OrchestratorResource { orchestrator });

    commands.insert_resource(GalaxySnapshot {
        edges: topology,
        planet_num,
        planet_states: lookup,
    });

    commands.insert_resource(GameState::WaitingStart);

    commands.insert_resource(GameTimer(Timer::from_seconds(
        AppConfig::get().game_tick_seconds,
        TimerMode::Repeating,
    )));

    commands.insert_resource(PlanetClickRes { planet: None });
}

#[derive(Resource, Deref, DerefMut)]
pub struct GameTimer(pub Timer);

pub fn game_loop(
    mut commands: Commands,
    mut orchestrator: ResMut<OrchestratorResource>,
    mut timer: ResMut<GameTimer>,
    state: Res<GameState>,
    time: Res<Time>,
) {
    if state.into_inner() == &GameState::Playing {
        timer.tick(time.delta());

        if timer.is_finished() {
            println!("ENTERED TIMER");
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
            // let _ = orchestrator.orchestrator.handle_game_messages();

            println!("EXITING TIMER");
            timer.reset();
        }
    }
}



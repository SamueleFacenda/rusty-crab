use std::collections::BTreeMap;

use bevy::prelude::Resource;
use common_game::utils::ID;

use crate::app::AppConfig;
use crate::orchestrator::{Orchestrator, PLANET_ORDER, PlanetType};

impl Orchestrator {
    pub fn get_planets_info(&self) -> PlanetInfoMap {
        let mut map = BTreeMap::new();
        // Check all the IDs, this usually is not reliable but the GUI can only have n planets from the config
        for id in 1..=AppConfig::get().number_of_planets {
            match self.get_planet_state(id as ID) {
                Some(Ok(state)) => {
                    map.insert(id, PlanetInfo {
                        status: Status::Running,
                        energy_cells: state.energy_cells,
                        charged_cells_count: state.charged_cells_count,
                        rocket: state.has_rocket,
                        name: PLANET_ORDER[(id as usize - 1) % PLANET_ORDER.len()]
                    });
                }
                Some(Err(e)) => {
                    log::error!("Error getting state for planet {id}: {e}");
                }
                None => {
                    // Planet not found: already destroyed
                    map.insert(id, PlanetInfo {
                        status: Status::Dead,
                        energy_cells: vec![],
                        charged_cells_count: 0,
                        rocket: false,
                        name: PLANET_ORDER[0]
                    });
                }
            }
        }

        PlanetInfoMap { map }
    }
}

#[derive(Resource)]
pub struct OrchestratorResource {
    pub orchestrator: Orchestrator
}

#[derive(Resource, PartialEq, Eq)]
pub enum GameState {
    WaitingStart,
    Playing,
    Paused
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Status {
    Running,
    Paused,
    Dead
}

#[derive(PartialEq, Debug, Clone)]
pub struct PlanetInfo {
    pub status: Status,
    pub energy_cells: Vec<bool>,
    pub charged_cells_count: usize,
    pub rocket: bool,
    pub name: PlanetType
}

#[derive(Clone)]
pub struct PlanetInfoMap {
    map: BTreeMap<u32, PlanetInfo>
}

impl PlanetInfoMap {
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &PlanetInfo)> { self.map.iter() }

    pub(crate) fn get_info(&self, id: u32) -> Option<PlanetInfo> { self.map.get(&id).cloned() }
}

#[derive(Resource, Clone)]
pub struct GalaxySnapshot {
    pub edges: Vec<(u32, u32)>,
    pub planet_num: u32,
    pub planet_states: PlanetInfoMap
}

pub struct SelectedPlanet {
    pub id: u32,
    pub name: PlanetType
}

#[derive(Resource)]
pub struct PlanetClickRes {
    pub planet: Option<SelectedPlanet>
}

pub enum OrchestratorEvent {
    PlanetDestroyed { planet_id: u32 },
    SunraySent { planet_id: u32 },
    SunrayReceived { planet_id: u32 },
    AsteroidSent { planet_id: u32 },
    ExplorerMoved { origin: u32, destination: u32 }
}

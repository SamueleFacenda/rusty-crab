use std::collections::BTreeMap;

use bevy::prelude::Resource;

use crate::orchestrator::{Orchestrator, PlanetType};

impl Orchestrator {
    pub fn get_planets_info(&self) -> PlanetInfoMap {
        // Placeholder implementation
        PlanetInfoMap {
            map: BTreeMap::new(),
        }
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
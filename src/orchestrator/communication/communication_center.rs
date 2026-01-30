use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, ExplorerToOrchestratorKind, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{
    OrchestratorToPlanet, PlanetToOrchestrator, PlanetToOrchestratorKind,
};
use common_game::utils::ID;
use std::collections::HashMap;

use super::{
    ExplorerChannelDemultiplexer, ExplorerLoggingSender, PlanetChannelDemultiplexer,
    PlanetLoggingSender,
};
use crate::explorers::BagContent;

/// Like a control tower, this struct provides utilities and logic handling for communication
pub(crate) struct CommunicationCenter {
    // List of explorers
    pub to_explorers: HashMap<ID, ExplorerLoggingSender>,
    // List of planets
    pub to_planets: HashMap<ID, PlanetLoggingSender>,

    pub planets_rx: PlanetChannelDemultiplexer,
    pub explorers_rx: ExplorerChannelDemultiplexer,
}

impl CommunicationCenter {
    pub fn new(
        to_explorers: HashMap<ID, ExplorerLoggingSender>,
        to_planets: HashMap<ID, PlanetLoggingSender>,
        planets_rx: PlanetChannelDemultiplexer,
        explorers_rx: ExplorerChannelDemultiplexer,
    ) -> Self {
        CommunicationCenter {
            to_explorers,
            to_planets,
            planets_rx,
            explorers_rx,
        }
    }

    pub fn send_to_planet(
        &self,
        planet_id: ID,
        msg: OrchestratorToPlanet,
    ) -> Result<(), String> {
        self.to_planets[&planet_id]
            .send(msg, planet_id)
            .map_err(|e| e.to_string())
    }

    pub fn send_to_explorer(
        &self,
        explorer_id: ID,
        msg: OrchestratorToExplorer,
    ) -> Result<(), String> {
        self.to_explorers[&explorer_id]
            .send(msg, explorer_id)
            .map_err(|e| e.to_string())
    }

    pub fn planet_req_ack(
        &mut self,
        planet_id: ID,
        msg: OrchestratorToPlanet,
        expected: PlanetToOrchestratorKind,
    ) -> Result<PlanetToOrchestrator, String> {
        self.send_to_planet(planet_id, msg)?;
        self.recv_from_planet(planet_id).map(|res| {
            if PlanetToOrchestratorKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected planet {planet_id} to respond with {expected:?}, but got {res:?}"
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    pub fn explorer_req_ack(
        &mut self,
        explorer_id: ID,
        msg: OrchestratorToExplorer,
        expected: ExplorerToOrchestratorKind,
    ) -> Result<ExplorerToOrchestrator<BagContent>, String> {
        self.send_to_explorer(explorer_id, msg)?;
        self.recv_from_explorer(explorer_id).map(|res| {
            if ExplorerToOrchestratorKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected explorer {explorer_id} to respond with {expected:?}, but got {res:?}"
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    pub fn recv_from_explorer(
        &mut self,
        explorer_id: ID,
    ) -> Result<ExplorerToOrchestrator<BagContent>, String> {
        self.explorers_rx.recv_from(explorer_id)
    }

    pub fn recv_from_planet(&mut self, planet_id: ID) -> Result<PlanetToOrchestrator, String> {
        self.planets_rx.recv_from(planet_id)
    }
}

use common_game::protocols::planet_explorer::{
    ExplorerToPlanet, PlanetToExplorer, PlanetToExplorerKind,
};
use common_game::utils::ID;
use std::collections::HashMap;
use crate::app::AppConfig;
use super::{PlanetLoggingSender, PlanetLoggingReceiver };

/// Like a control tower, this struct provides utilities and logic handling for communication
pub(super) struct PlanetsCommunicator {
    pub to_planets: HashMap<ID, PlanetLoggingSender>,
    pub planets_rx: PlanetLoggingReceiver,
}

impl PlanetsCommunicator {
    pub fn new(
        to_planets: HashMap<ID, PlanetLoggingSender>,
        planets_rx: PlanetLoggingReceiver,
    ) -> Self {
        PlanetsCommunicator {
            to_planets,
            planets_rx,
        }
    }

    pub fn add_planet(&mut self, planet_id: ID, mut sender: PlanetLoggingSender) {
        sender.set_other_id(planet_id);
        self.to_planets.insert(planet_id, sender);
    }

    pub fn set_current_planet(&mut self, planet_id: ID) {
        self.planets_rx.set_other_id(planet_id);
    }
    
    

    fn send_to_planet(&self, planet_id: ID, msg: ExplorerToPlanet) -> Result<(), String> {
        self.to_planets[&planet_id]
            .send(msg)
            .map_err(|e| e.to_string())
    }

    fn planet_req_ack(
        &mut self,
        planet_id: ID,
        msg: ExplorerToPlanet,
        expected: PlanetToExplorerKind,
    ) -> Result<PlanetToExplorer, String> {
        self.send_to_planet(planet_id, msg)?;
        self.recv_from_planet().map(|res| {
            if PlanetToExplorerKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected planet {planet_id} to respond with {expected:?}, but got {res:?}"
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    fn recv_from_planet(&mut self) -> Result<PlanetToExplorer, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);
        self.planets_rx.recv_timeout(timeout)
            .map_err(|e| format!("Error waiting for message from planet: {e}"))
    }
}

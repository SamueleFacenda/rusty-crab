use common_game::protocols::planet_explorer::{
    ExplorerToPlanet, PlanetToExplorer, PlanetToExplorerKind,
};
use common_game::utils::ID;
use std::collections::{HashMap, HashSet};
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType, GenericResource};
use crate::app::AppConfig;
use super::{PlanetLoggingSender, PlanetLoggingReceiver };

/// Like a control tower, this struct provides utilities and logic handling for communication
pub(super) struct PlanetsCommunicator {
    to_planets: HashMap<ID, PlanetLoggingSender>,
    planets_rx: PlanetLoggingReceiver,
    explorer_id: ID,
}

impl PlanetsCommunicator {
    pub fn new(
        planets_rx: PlanetLoggingReceiver,
        explorer_id: ID,
    ) -> Self {
        PlanetsCommunicator {
            to_planets: HashMap::new(),
            planets_rx,
            explorer_id,
        }
    }

    pub fn add_planet(&mut self, planet_id: ID, mut sender: PlanetLoggingSender) {
        sender.set_other_id(planet_id);
        self.to_planets.insert(planet_id, sender);
    }

    pub fn set_current_planet(&mut self, planet_id: ID) {
        self.planets_rx.set_other_id(planet_id);
    }

    pub fn basic_resource_discovery(&self, planet_id: ID) -> Result<HashSet<BasicResourceType>, String> {
        Ok(self.req_ack(
            planet_id,
            ExplorerToPlanet::SupportedResourceRequest {explorer_id: self.explorer_id},
            PlanetToExplorerKind::SupportedResourceResponse)?
            .into_supported_resource_response().unwrap()) // Safe unwrap since we checked the kind
    }

    pub fn combination_rules_discovery(&self, planet_id: ID) -> Result<HashSet<ComplexResourceType>, String> {
        Ok(self.req_ack(
            planet_id,
            ExplorerToPlanet::SupportedCombinationRequest {explorer_id: self.explorer_id},
            PlanetToExplorerKind::SupportedCombinationResponse)?
            .into_supported_combination_response().unwrap()) // Safe unwrap since we checked the kind
    }

    pub fn generate_basic_resource(&self, planet_id: ID, resource: BasicResourceType) -> Result<Option<BasicResource>, String> {
        Ok(self.req_ack(
            planet_id,
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id: self.explorer_id,
                resource,
            },
            PlanetToExplorerKind::GenerateResourceResponse)?
            .into_generate_resource_response().unwrap()) // Safe unwrap since we checked the kind
    }

    /// The first result is the communication result, the second is the actual response from the planet
    pub fn combine_resources(&self, planet_id: ID, msg: ComplexResourceRequest) -> Result<Result<ComplexResource, (String, GenericResource, GenericResource)>, String> {
        Ok(self.req_ack(
            planet_id,
            ExplorerToPlanet::CombineResourceRequest {
                explorer_id: self.explorer_id,
                msg,
            },
            PlanetToExplorerKind::CombineResourceResponse)?
            .into_combine_resource_response().unwrap()) // Safe unwrap since we checked the kind
    }

    pub fn get_available_energy_cells_num(&self, planet_id: ID) -> Result<u32, String> {
        Ok(self.req_ack(
            planet_id,
            ExplorerToPlanet::AvailableEnergyCellRequest {explorer_id: self.explorer_id},
            PlanetToExplorerKind::AvailableEnergyCellResponse)?
            .into_available_energy_cell_response().unwrap()) // Safe unwrap since we checked the kind
    }

    fn send(&self, planet_id: ID, msg: ExplorerToPlanet) -> Result<(), String> {
        self.to_planets[&planet_id]
            .send(msg)
            .map_err(|e| e.to_string())
    }

    fn req_ack(
        &self,
        planet_id: ID,
        msg: ExplorerToPlanet,
        expected: PlanetToExplorerKind,
    ) -> Result<PlanetToExplorer, String> {
        self.send(planet_id, msg)?;
        self.recv_timeout().map(|res| {
            if PlanetToExplorerKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected planet {planet_id} to respond with {expected:?}, but got {res:?}"
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    fn recv_timeout(&self) -> Result<PlanetToExplorer, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);
        self.planets_rx.recv_timeout(timeout)
            .map_err(|e| format!("Error waiting for message from planet: {e}"))
    }
}

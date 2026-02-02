use common_game::components::asteroid::Asteroid;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestratorKind, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::utils::ID;

use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;
use crate::orchestrator::{OrchestratorManualAction, OrchestratorState};

pub(crate) struct ManualUpdateStrategy<'a> {
    state: &'a mut OrchestratorState
}

#[allow(dead_code)] // not all functions are implemented in GUI, but they will be in the future
impl ManualUpdateStrategy<'_> {
    pub fn new(state: &'_ mut OrchestratorState) -> ManualUpdateStrategy<'_> { ManualUpdateStrategy { state } }

    fn basic_resource_discovery(&mut self, explorer_id: ID) -> Result<(), String> {
        self.check_explorer_id(explorer_id)?;

        let (explorer_id, basic_resources) = self
            .state
            .explorers_communication_center
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::SupportedResourceRequest,
                ExplorerToOrchestratorKind::SupportedResourceResult
            )?
            .into_supported_resource_result()
            .unwrap(); // Unwrap is safe due to expected kind

        if basic_resources.is_empty() {
            return Err(format!(
                "A SupportedResourceRequest from explorer {explorer_id} returned that a planet produces no basic \
                 resource"
            ));
        }
        Ok(())
    }

    fn combination_resource_discovery(&mut self, explorer_id: ID) -> Result<(), String> {
        self.check_explorer_id(explorer_id)?;

        let _ = self
            .state
            .explorers_communication_center
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::SupportedCombinationRequest,
                ExplorerToOrchestratorKind::SupportedCombinationResult
            )?
            .into_supported_combination_result()
            .unwrap(); // Unwrap is safe due to expected kind
        Ok(())
    }

    fn basic_resource_generation(&mut self, explorer_id: ID, resource: BasicResourceType) -> Result<(), String> {
        self.check_explorer_id(explorer_id)?;

        let result = self
            .state
            .explorers_communication_center
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::GenerateResourceRequest { to_generate: resource },
                ExplorerToOrchestratorKind::GenerateResourceResponse
            )?
            .into_generate_resource_response()
            .unwrap()
            .1; // Unwrap is safe due to expected kind

        if result.is_ok() {
            self.state.gui_events_buffer.basic_resource_generated(explorer_id, resource);
            *self
                .state
                .explorer_bags
                .entry(explorer_id)
                .or_default()
                .content
                .entry(ResourceType::Basic(resource))
                .or_default() += 1;
        }

        if result.is_err() {
            log::error!(
                "Basic resource generation failed for explorer {explorer_id} trying to create {resource:?}, going on."
            );
            // return Err(format!(
            //     "Basic resource from explorer {exp_id} request has not been generated"
            // ));
        }
        Ok(())
    }

    fn resource_combination(&mut self, explorer_id: ID, complex: ComplexResourceType) -> Result<(), String> {
        self.check_explorer_id(explorer_id)?;

        let result = self
            .state
            .explorers_communication_center
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::CombineResourceRequest { to_generate: complex },
                ExplorerToOrchestratorKind::CombineResourceResponse
            )?
            .into_combine_resource_response()
            .unwrap()
            .1; // Unwrap is safe due to expected kind

        if result.is_ok() {
            self.state.gui_events_buffer.complex_resource_generated(explorer_id, complex);
            let bag = self.state.explorer_bags.entry(explorer_id).or_default();
            *bag.content.entry(ResourceType::Complex(complex)).or_default() += 1;
            let (a, b) = get_recipe(complex);
            bag.content.entry(a).and_modify(|qty| *qty -= 1);
            bag.content.entry(b).and_modify(|qty| *qty -= 1);
        }

        if result.is_err() {
            log::error!(
                "Resource combination failed for explorer {explorer_id} trying to create {complex:?}, going on."
            );
            // return Err(format!(
            //     "Basic resource from explorer {exp_id} request has not been generated"
            // ));
        }
        Ok(())
    }

    fn handle_travel_request(&mut self, explorer_id: ID, dst_planet_id: ID) -> Result<(), String> {
        self.check_planet_id(dst_planet_id)?;
        self.check_explorer_id(explorer_id)?;

        let current_planet_id = self.state.explorers[&explorer_id].current_planet;

        // Communicate invalid travel if planets are not connected
        if !self.state.galaxy.are_planets_connected(current_planet_id, dst_planet_id) {
            return Err(format!(
                "This travel request cannot be granted because the destination planet {dst_planet_id}is not directly \
                 linked to the current one ({current_planet_id})"
            ));
        }

        let new_sender = self.state.explorers[&explorer_id].tx_planet.clone();
        self.state
            .planets_communication_center
            .notify_planet_incoming_explorer(explorer_id, dst_planet_id, new_sender)?;
        self.state.planets_communication_center.notify_planet_explorer_left(explorer_id, current_planet_id)?;
        let new_sender = self.state.planets[&dst_planet_id].tx_explorer.clone();
        self.state
            .explorers_communication_center
            .notify_explorer_successful_movement(explorer_id, dst_planet_id, new_sender)?;

        // Update internal state
        self.state
            .explorers
            .get_mut(&explorer_id)
            .unwrap() // It is checked above that the explorer exists
            .current_planet = dst_planet_id;
        self.state.gui_events_buffer.explorer_moved(explorer_id, dst_planet_id);

        Ok(())
    }

    fn handle_send_asteroid(&mut self, planet_id: ID) -> Result<(), String> {
        self.check_planet_id(planet_id)?;
        self.state.gui_events_buffer.asteroid_sent(planet_id);
        let rocket = self
            .state
            .planets_communication_center
            .req_ack(
                planet_id,
                OrchestratorToPlanet::Asteroid(Asteroid::default()),
                PlanetToOrchestratorKind::AsteroidAck
            )?
            .into_asteroid_ack()
            .unwrap()
            .1; // Unwrap is safe due to expected kind

        if rocket.is_none() {
            self.state.handle_planet_destroyed(planet_id)?;
        }

        Ok(())
    }

    fn handle_send_sunray(&mut self, planet_id: ID) -> Result<(), String> {
        self.check_planet_id(planet_id)?;
        self.state.gui_events_buffer.sunray_sent(planet_id);
        self.state
            .planets_communication_center
            .req_ack(planet_id, OrchestratorToPlanet::Sunray(Sunray::default()), PlanetToOrchestratorKind::SunrayAck)?
            .into_sunray_ack()
            .unwrap(); // Unwrap is safe due to expected kind

        self.state.gui_events_buffer.sunray_received(planet_id);
        Ok(())
    }

    fn check_planet_id(&self, id: ID) -> Result<(), String> {
        if !self.state.planets.contains_key(&id) {
            return Err(format!("Planet with ID: {id} does not exist."));
        }
        Ok(())
    }

    fn check_explorer_id(&self, id: ID) -> Result<(), String> {
        if !self.state.explorers.contains_key(&id) {
            return Err(format!("Explorer with ID: {id} does not exist."));
        }
        Ok(())
    }
}

/// Returns the two resource types needed to create this complex resource.
pub fn get_recipe(complex: ComplexResourceType) -> (ResourceType, ResourceType) {
    match complex {
        ComplexResourceType::Water =>
            (ResourceType::Basic(BasicResourceType::Hydrogen), ResourceType::Basic(BasicResourceType::Oxygen)),
        ComplexResourceType::Diamond =>
            (ResourceType::Basic(BasicResourceType::Carbon), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Life =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Robot =>
            (ResourceType::Basic(BasicResourceType::Silicon), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::Dolphin =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::AIPartner =>
            (ResourceType::Complex(ComplexResourceType::Robot), ResourceType::Complex(ComplexResourceType::Diamond)),
    }
}

impl OrchestratorUpdateStrategy for ManualUpdateStrategy<'_> {
    fn update(&mut self) -> Result<(), String> {
        log::info!("Update called in manual mode. No automatic actions taken.");
        Ok(())
    }

    fn process_command(&mut self, command: OrchestratorManualAction) -> Result<(), String> {
        match command {
            OrchestratorManualAction::GenerateBasic { explorer_id, resource } =>
                self.basic_resource_generation(explorer_id, resource),
            OrchestratorManualAction::GenerateComplex { explorer_id, resource } =>
                self.resource_combination(explorer_id, resource),
            OrchestratorManualAction::SendAsteroid { planet_id } => self.handle_send_asteroid(planet_id),
            OrchestratorManualAction::SendSunray { planet_id } => self.handle_send_sunray(planet_id),
            OrchestratorManualAction::MoveExplorer { explorer_id, destination_planet_id } =>
                self.handle_travel_request(explorer_id, destination_planet_id),
        }
    }
}

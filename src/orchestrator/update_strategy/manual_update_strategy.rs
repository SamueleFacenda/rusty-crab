use std::fmt::format;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, ExplorerToOrchestratorKind, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::protocols::planet_explorer::PlanetToExplorerKind::SupportedResourceResponse;
use common_game::utils::ID;
use crate::orchestrator::OrchestratorState;
use crate::orchestrator::update_strategy::auto_update_strategy::AutoUpdateStrategy;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;

pub(crate) struct ManualUpdateStrategy<'a> {
    state: &'a mut OrchestratorState,
}

impl ManualUpdateStrategy<'_> {
    pub fn new(state: &'_ mut OrchestratorState) -> ManualUpdateStrategy<'_> {
        ManualUpdateStrategy { state }
    }

    fn basic_resource_discovery(
        &mut self,
        explorer_id: ID,
    ) -> Result<(), String> {
        self.check_explorer_id(&explorer_id)?;

        let (explorer_id, basic_resources) =
            self.state.communication_center.explorer_req_ack(
            explorer_id,
            OrchestratorToExplorer::SupportedResourceRequest,
            ExplorerToOrchestratorKind::SupportedResourceResult
        )?
            .into_supported_resource_result()
            .unwrap(); // Unwrap is safe due to expected kind

        if basic_resources.is_empty() {
            return Err(format!(
                "A SupportedResourceRequest from explorer {explorer_id} returned that a planet produces no basic resource"
            ));
        }
        Ok(())
    }

    fn combination_resource_discovery(
        &mut self,
        explorer_id: ID,
    ) -> Result<(), String> {
        self.check_explorer_id(&explorer_id)?;

        let (_, _) =
            self.state.communication_center.explorer_req_ack(
                explorer_id,
                OrchestratorToExplorer::SupportedCombinationRequest,
                ExplorerToOrchestratorKind::SupportedCombinationResult
            )?
                .into_supported_combination_result()
                .unwrap(); // Unwrap is safe due to expected kind
        Ok(())
    }

    fn basic_resource_generation(
        &mut self,
        explorer_id: ID,
        resource: BasicResourceType,
    ) -> Result<(), String> {
        self.check_explorer_id(&explorer_id)?;

        let (exp_id, result) =
            self.state.communication_center.explorer_req_ack(
                explorer_id,
                OrchestratorToExplorer::GenerateResourceRequest { to_generate: resource },
                ExplorerToOrchestratorKind::GenerateResourceResponse
            )?
                .into_generate_resource_response()
                .unwrap(); // Unwrap is safe due to expected kind

        // if result.is_err() {
        //     return Err(format!(
        //         "Basic resource from explorer {exp_id} request has not been generated"
        //     ));
        // }
        Ok(())
    }

    fn resource_combination(
        &mut self,
        explorer_id: ID,
        complex: ComplexResourceType
    ) -> Result<(), String> {
        self.check_explorer_id(&explorer_id)?;

        let (exp_id, result) =
            self.state.communication_center.explorer_req_ack(
                explorer_id,
                OrchestratorToExplorer::CombineResourceRequest { to_generate: complex },
                ExplorerToOrchestratorKind::CombineResourceResponse
            )?
                .into_combine_resource_response()
                .unwrap(); // Unwrap is safe due to expected kind

        // if result.is_err() {
        //     return Err(format!(
        //         "Basic resource from explorer {exp_id} request has not been generated"
        //     ));
        // }
        Ok(())
    }

    fn handle_travel_request(
        &mut self,
        explorer_id: ID,
        dst_planet_id: ID,
    ) -> Result<(), String> {
        self.check_planet_id(&dst_planet_id)?;
        self.check_explorer_id(&explorer_id)?;

        let current_planet_id = self.state.explorers[&explorer_id].current_planet;

        // Communicate invalid travel if planets are not connected
        if !self.state
            .galaxy
            .are_planets_connected(current_planet_id, dst_planet_id)
        {
            return Err(format!(
                "This travel request cannot be granted because the destination planet {dst_planet_id}\
                 is not directly linked to the current one ({current_planet_id})"
            ));
        }

        self.notify_planet_incoming_explorer(explorer_id, dst_planet_id)?;
        self.notify_planet_explorer_left(explorer_id, current_planet_id)?;
        self.notify_explorer_successful_movement(explorer_id, dst_planet_id)?;

        // Update internal state
        self.state
            .explorers
            .get_mut(&explorer_id)
            .unwrap()  // It is checked above that the explorer exists
            .current_planet = dst_planet_id;

        Ok(())
    }

    fn notify_planet_incoming_explorer(
        &mut self,
        explorer_id: ID,
        dst_planet_id: ID,
    ) -> Result<(), String> {
        let new_sender = self.state.explorers[&explorer_id].tx_planet.clone();
        let (_, accepted_explorer_id, res) = self.state
            .communication_center
            .planet_req_ack(
                dst_planet_id,
                OrchestratorToPlanet::IncomingExplorerRequest {
                    explorer_id,
                    new_sender,
                },
                PlanetToOrchestratorKind::IncomingExplorerResponse,
            )?
            .into_incoming_explorer_response()
            .unwrap(); // Unwrap is safe due to expected kind

        if res.is_err() {
            return Err(format!(
                "Planet {dst_planet_id} failed to accept incoming explorer {explorer_id}: {}",
                res.err().unwrap()
            ));
        }

        if accepted_explorer_id != explorer_id {
            return Err(format!(
                "Planet {dst_planet_id} accepted incoming explorer {accepted_explorer_id}, but was expected to accept explorer {explorer_id}"
            ));
        }
        Ok(())
    }

    fn notify_planet_explorer_left(
        &mut self,
        explorer_id: ID,
        current_planet_id: ID,
    ) -> Result<(), String> {
        let (_, left_explorer_id, res) = self.state
            .communication_center
            .planet_req_ack(
                current_planet_id,
                OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id },
                PlanetToOrchestratorKind::OutgoingExplorerResponse,
            )?
            .into_outgoing_explorer_response()
            .unwrap(); // Unwrap is safe due to expected kind

        if res.is_err() {
            return Err(format!(
                "Planet {current_planet_id} failed to confirm outgoing explorer {explorer_id}: {}",
                res.err().unwrap()
            ));
        }

        if left_explorer_id != explorer_id {
            return Err(format!(
                "Planet {current_planet_id} confirmed outgoing explorer {left_explorer_id}, but was expected to confirm explorer {explorer_id}"
            ));
        }
        Ok(())
    }

    fn notify_explorer_successful_movement(
        &mut self,
        explorer_id: ID,
        planet_id: ID,
    ) -> Result<(), String> {
        let sender_to_new_planet = Some(self.state.planets[&planet_id].tx_explorer.clone());
        let new_planet_id = self.state
            .communication_center
            .explorer_req_ack(
                explorer_id,
                OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet: sender_to_new_planet.clone(),
                    planet_id,
                },
                ExplorerToOrchestratorKind::MovedToPlanetResult,
            )?
            .into_moved_to_planet_result()
            .unwrap()
            .1; // Unwrap is safe due to expected kind

        if new_planet_id != planet_id {
            return Err(format!(
                "Explorer {explorer_id} moved to planet {new_planet_id}, but was expected to move to planet {planet_id}"
            ));
        }
        Ok(())
    }

    fn check_planet_id(&self, id: &ID) -> Result<(), String> {
        if !self.state.planets.contains_key(&id){
            return Err(format!(
                "Planet with ID: {id} does not exist."
            ))
        }
        Ok(())
    }

    fn check_explorer_id(&self, id: &ID) -> Result<(), String> {
        if !self.state.explorers.contains_key(&id){
            return Err(format!(
                "Explorer with ID: {id} does not exist."
            ));
        }
        Ok(())
    }
}

impl OrchestratorUpdateStrategy for ManualUpdateStrategy<'_> {
    fn update(&mut self) -> Result<(), String> {
        log::info!("Update called in manual mode. No automatic actions taken.");
        Ok(())
    }

    fn process_commands(&mut self) -> Result<(), String> {
        Ok(()) // TODO
    }
}

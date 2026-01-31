use std::fmt::format;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestratorKind, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::utils::ID;
use crate::orchestrator::OrchestratorState;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;

pub(crate) struct ManualUpdateStrategy;

impl ManualUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        // In manual mode, we do not perform any automatic updates.
        Ok(())
    }

    fn handle_travel_request(
        &self,
        explorer_id: ID,
        current_planet_id: ID,
        dst_planet_id: ID,
        state: &mut OrchestratorState,
    ) -> Result<(), String> {
        check_planet_id(&current_planet_id, state)?;
        check_planet_id(&dst_planet_id, state)?;
        check_explorer_id(&explorer_id, state)?;

        if current_planet_id != state.explorers[&explorer_id].current_planet {
            return Err(format!(
                "Explorer {explorer_id} requested travel from planet {current_planet_id}, but is currently on planet {}",
                state.explorers[&explorer_id].current_planet
            ));
        }

        // Communicate invalid travel if planets are not connected
        if !state
            .galaxy
            .are_planets_connected(current_planet_id, dst_planet_id)
        {
            return Err(format!(
                "This travel request cannot be granted because the destination planet {dst_planet_id}\
                 is not directly linked to the current one ({current_planet_id})"
            ));
        }

        self.notify_planet_incoming_explorer(explorer_id, dst_planet_id, state)?;
        self.notify_planet_explorer_left(explorer_id, current_planet_id, state)?;
        self.notify_explorer_successful_movement(explorer_id, dst_planet_id, state)?;

        // Update internal state
        state
            .explorers
            .get_mut(&explorer_id)
            .unwrap()
            .current_planet = dst_planet_id;

        Ok(())
    }

    #[allow(clippy::unused_self)] // these are better as instance methods
    fn notify_planet_incoming_explorer(
        &self,
        explorer_id: ID,
        dst_planet_id: ID,
        state: &mut OrchestratorState,
    ) -> Result<(), String> {
        let new_sender = state.explorers[&explorer_id].tx_planet.clone();
        let (_, accepted_explorer_id, res) = state
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

    #[allow(clippy::unused_self)] // these are better as instance methods
    fn notify_planet_explorer_left(
        &self,
        explorer_id: ID,
        current_planet_id: ID,
        state: &mut OrchestratorState,
    ) -> Result<(), String> {
        let (_, left_explorer_id, res) = state
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

    #[allow(clippy::unused_self)] // these are better as instance methods
    fn notify_explorer_successful_movement(
        &self,
        explorer_id: ID,
        planet_id: ID,
        state: &mut OrchestratorState,
    ) -> Result<(), String> {
        let sender_to_new_planet = Some(state.planets[&planet_id].tx_explorer.clone());
        let new_planet_id = state
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






}

fn check_planet_id(id: &ID, state: &OrchestratorState) -> Result<(), String> {
    if !state.planets.contains_key(&id){
        return Err(format!(
            "Planet with ID: {id} does not exist."
        ))
    }
    Ok(())
}

fn check_explorer_id(id: &ID, state: &OrchestratorState) -> Result<(), String> {
    if !state.explorers.contains_key(&id){
        return Err(format!(
            "Explorer with ID: {id} does not exist."
        ));
    }
    Ok(())
}

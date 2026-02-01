use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestratorKind, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::Sender;
use crate::orchestrator::communication::ExplorerCommunicationCenter;

impl ExplorerCommunicationCenter {
    pub fn notify_explorer_successful_movement(
        &mut self,
        explorer_id: ID,
        planet_id: ID,
        new_sender: Sender<ExplorerToPlanet>
    ) -> Result<(), String> {
        let new_planet_id = self
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet: Some(new_sender),
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
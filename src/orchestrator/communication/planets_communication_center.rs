use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::protocols::planet_explorer::PlanetToExplorer;
use common_game::utils::ID;
use crossbeam_channel::Sender;

use crate::orchestrator::communication::PlanetCommunicationCenter;

impl PlanetCommunicationCenter {
    pub(crate) fn notify_planet_incoming_explorer(
        &mut self,
        explorer_id: ID,
        dst_planet_id: ID,
        new_sender: Sender<PlanetToExplorer>
    ) -> Result<(), String> {
        let (_, accepted_explorer_id, res) = self
            .req_ack(
                dst_planet_id,
                OrchestratorToPlanet::IncomingExplorerRequest { explorer_id, new_sender },
                PlanetToOrchestratorKind::IncomingExplorerResponse
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
                "Planet {dst_planet_id} accepted incoming explorer {accepted_explorer_id}, but was expected to accept \
                 explorer {explorer_id}"
            ));
        }
        Ok(())
    }

    pub(crate) fn notify_planet_explorer_left(&mut self, explorer_id: ID, current_planet_id: ID) -> Result<(), String> {
        let (_, left_explorer_id, res) = self
            .req_ack(
                current_planet_id,
                OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id },
                PlanetToOrchestratorKind::OutgoingExplorerResponse
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
                "Planet {current_planet_id} confirmed outgoing explorer {left_explorer_id}, but was expected to \
                 confirm explorer {explorer_id}"
            ));
        }
        Ok(())
    }
}

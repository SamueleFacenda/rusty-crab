use std::collections::{HashMap, HashSet};

use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer,
                                                    OrchestratorToExplorerKind};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::Sender;

use super::{OrchestratorLoggingReceiver, OrchestratorLoggingSender};
use crate::app::AppConfig;
use crate::explorers::BagContent;

/// API for communication between an explorer and the orchestrator
pub(crate) struct OrchestratorCommunicator {
    orchestrator_tx: OrchestratorLoggingSender,
    orchestrator_rx: OrchestratorLoggingReceiver,
    explorer_id: ID
}

impl OrchestratorCommunicator {
    pub fn new(
        orchestrator_tx: OrchestratorLoggingSender,
        orchestrator_rx: OrchestratorLoggingReceiver,
        explorer_id: ID
    ) -> Self {
        OrchestratorCommunicator { orchestrator_tx, orchestrator_rx, explorer_id }
    }

    pub fn recv(&self) -> Result<OrchestratorToExplorer, String> {
        self.orchestrator_rx.recv().map_err(|e| e.to_string())
    }

    pub fn discover_neighbors(&self, current_planet_id: ID) -> Result<Vec<ID>, String> {
        Ok(self
            .req_ack(
                ExplorerToOrchestrator::NeighborsRequest { explorer_id: self.explorer_id, current_planet_id },
                OrchestratorToExplorerKind::NeighborsResponse
            )?
            .into_neighbors_response()
            .unwrap()) // Safe unwrap since we checked the kind
    }

    /// Returns Ok(None) if the travel was not possible (e.g. planet destroyed)
    pub fn travel_to_planet(
        &self,
        current_planet_id: ID,
        dst_planet_id: ID
    ) -> Result<Option<Sender<ExplorerToPlanet>>, String> {
        let sender = self
            .req_ack(
                ExplorerToOrchestrator::TravelToPlanetRequest {
                    explorer_id: self.explorer_id,
                    current_planet_id,
                    dst_planet_id
                },
                OrchestratorToExplorerKind::MoveToPlanet
            )?
            .into_move_to_planet()
            .unwrap()
            .0;

        self.send_move_ack(if sender.is_some() { dst_planet_id } else { current_planet_id })?;
        Ok(sender)
    }

    pub fn send_current_planet_ack(&self, planet_id: ID) -> Result<(), String> {
        self.orchestrator_tx
            .send(ExplorerToOrchestrator::CurrentPlanetResult { explorer_id: self.explorer_id, planet_id })
    }

    pub fn send_start_ack(&self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: self.explorer_id })
    }

    pub fn send_reset_ack(&self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id: self.explorer_id })
    }

    pub fn send_stop_ack(&self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::StopExplorerAIResult { explorer_id: self.explorer_id })
    }

    pub fn send_move_ack(&self, planet_id: ID) -> Result<(), String> {
        self.orchestrator_tx
            .send(ExplorerToOrchestrator::MovedToPlanetResult { explorer_id: self.explorer_id, planet_id })
    }

    pub fn send_bag_content_ack(&self, bag_content: BagContent) -> Result<(), String> {
        self.orchestrator_tx
            .send(ExplorerToOrchestrator::BagContentResponse { explorer_id: self.explorer_id, bag_content })
    }

    pub fn send_supported_resources_ack(&self, supported_resources: HashSet<BasicResourceType>) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::SupportedResourceResult {
            explorer_id: self.explorer_id,
            supported_resources
        })
    }

    pub fn send_combination_rules_ack(&self, combination_rules: HashSet<ComplexResourceType>) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::SupportedCombinationResult {
            explorer_id: self.explorer_id,
            combination_list: combination_rules
        })
    }

    pub fn send_generation_ack(&self, res: Result<(), String>) -> Result<(), String> {
        self.orchestrator_tx
            .send(ExplorerToOrchestrator::GenerateResourceResponse { explorer_id: self.explorer_id, generated: res })
    }

    pub fn send_combination_ack(&self, res: Result<(), String>) -> Result<(), String> {
        self.orchestrator_tx
            .send(ExplorerToOrchestrator::CombineResourceResponse { explorer_id: self.explorer_id, generated: res })
    }

    pub fn send_kill_ack(&self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::KillExplorerResult { explorer_id: self.explorer_id })
    }

    fn send(&self, msg: ExplorerToOrchestrator<BagContent>) -> Result<(), String> {
        self.orchestrator_tx.send(msg).map_err(|e| e.to_string())
    }

    fn req_ack(
        &self,
        msg: ExplorerToOrchestrator<BagContent>,
        expected: OrchestratorToExplorerKind
    ) -> Result<OrchestratorToExplorer, String> {
        self.send(msg)?;
        self.recv_timeout().map(|res| {
            if OrchestratorToExplorerKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!("Expected orchestrator to respond with {expected:?}, but got {res:?}"))
            }
        })? // Flatten the Result<Result<...>>
    }

    fn recv_timeout(&self) -> Result<OrchestratorToExplorer, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);
        self.orchestrator_rx
            .recv_timeout(timeout)
            .map_err(|e| format!("Error waiting for message from orchestrator: {e}"))
    }
}

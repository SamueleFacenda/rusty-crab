use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer, OrchestratorToExplorerKind,
};
use common_game::utils::ID;
use std::collections::{HashMap, HashSet};
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResourceType};
use crate::app::AppConfig;
use super::{OrchestratorLoggingSender, OrchestratorLoggingReceiver };
use crate::explorers::BagContent;

/// API for communication between an explorer and the orchestrator
pub(super) struct OrchestratorCommunicator {
    orchestrator_tx: OrchestratorLoggingSender,
    orchestrator_rx: OrchestratorLoggingReceiver,
    explorer_id: ID,
}

impl OrchestratorCommunicator {
    pub fn new(
        orchestrator_tx: OrchestratorLoggingSender,
        orchestrator_rx: OrchestratorLoggingReceiver,
        explorer_id: ID,
    ) -> Self {
        OrchestratorCommunicator {
            orchestrator_tx,
            orchestrator_rx,
            explorer_id,
        }
    }

    fn recv(&mut self) -> Result<OrchestratorToExplorer, String> {
        self.orchestrator_rx.recv()
            .map_err(|e| e.to_string())
    }

    pub fn discover_neighbors(&mut self, current_planet_id: ID) -> Result<Vec<ID>, String> {
        Ok(self.req_ack(ExplorerToOrchestrator::NeighborsRequest {
            explorer_id: self.explorer_id,
            current_planet_id},
            OrchestratorToExplorerKind::NeighborsResponse
        )?.into_neighbors_response().unwrap()) // Safe unwrap since we checked the kind
    }

    pub fn send_reset_ack(&mut self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::ResetExplorerAIResult {
            explorer_id: self.explorer_id,
        })
    }

    pub fn send_stop_ack(&mut self) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::StopExplorerAIResult {
            explorer_id: self.explorer_id,
        })
    }

    pub fn send_move_ack(&mut self, planet_id: ID) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::MovedToPlanetResult {
            explorer_id: self.explorer_id,
            planet_id
        })
    }

    pub fn send_bag_content_ack(&mut self, bag_content: BagContent) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::BagContentResponse {
            explorer_id: self.explorer_id,
            bag_content,
        })
    }

    pub fn send_supported_resources_ack(&mut self, supported_resources: HashSet<BasicResourceType>, ) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::SupportedResourceResult {
            explorer_id: self.explorer_id,
            supported_resources,
        })
    }

    pub fn send_combination_rules_ack(&mut self, combination_rules: HashSet<ComplexResourceType> ) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::SupportedCombinationResult {
            explorer_id: self.explorer_id,
            combination_list: combination_rules,
        })
    }

    pub fn send_generation_ack(&mut self, res: Result<(), String>) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::GenerateResourceResponse {
            explorer_id: self.explorer_id,
            generated: res,
        })
    }

    pub fn send_combination_ack(&mut self, res: Result<(), String>) -> Result<(), String> {
        self.orchestrator_tx.send(ExplorerToOrchestrator::CombineResourceResponse {
            explorer_id: self.explorer_id,
            generated: res,
        })
    }

    fn send(
        &self,
        msg: ExplorerToOrchestrator<BagContent>,
    ) -> Result<(), String> {
        self.orchestrator_tx
            .send(msg)
            .map_err(|e| e.to_string())
    }

    fn req_ack(
        &mut self,
        msg: ExplorerToOrchestrator<BagContent>,
        expected: OrchestratorToExplorerKind,
    ) -> Result<OrchestratorToExplorer, String> {
        self.send(msg)?;
        self.recv_timeout().map(|res| {
            if OrchestratorToExplorerKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected orchestrator to respond with {expected:?}, but got {res:?}"
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    fn recv_timeout(&mut self) -> Result<OrchestratorToExplorer, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);
        self.orchestrator_rx.recv_timeout(timeout)
            .map_err(|e| format!("Error waiting for message from orchestrator: {e}"))
    }
}

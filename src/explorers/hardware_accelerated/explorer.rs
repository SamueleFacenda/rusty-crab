use std::collections::HashMap;
use common_game::components::resource::{GenericResource, ResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use log::info;

use crate::explorers::hardware_accelerated::communication::PlanetLoggingSender;
use crate::explorers::hardware_accelerated::probability_estimator::ProbabilityEstimator;
use crate::explorers::hardware_accelerated::round_executor::RoundExecutor;
use crate::explorers::hardware_accelerated::{GalaxyKnowledge, OrchestratorCommunicator, OrchestratorLoggingReceiver,
                                             OrchestratorLoggingSender, PlanetLoggingReceiver, PlanetsCommunicator};
use crate::explorers::{BagContent, Explorer};

// DTO for the explorer's state
pub(super) struct ExplorerState {
    pub bag: Bag,
    pub knowledge: Option<GalaxyKnowledge>,
    pub current_planet: ID,
    pub asteroid_probability_estimator: ProbabilityEstimator,
    pub sunray_probability_estimator: ProbabilityEstimator
}

pub struct HardwareAcceleratedExplorer {
    id: ID,

    state: ExplorerState,

    orchestrator_communicator: OrchestratorCommunicator,
    planets_communicator: PlanetsCommunicator
}

struct Bag {
    res: HashMap<ResourceType, Vec<GenericResource>>
}

impl Bag {
    pub fn to_bag_content(&self) -> BagContent {
        let mut content = BagContent::default();
        for resources in self.res.values() {
            for resource in resources {
                content.res.push(resource.get_type());
            }
        }
        content
    }
}


struct ExplorerKnowledge {
    // not implemented in example
}

impl Explorer for HardwareAcceleratedExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_current_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>
    ) -> Self {
        HardwareAcceleratedExplorer {
            id,
            state: ExplorerState {
                bag: Bag { res: HashMap::new() },
                knowledge: None,
                current_planet,
                asteroid_probability_estimator: ProbabilityEstimator::new(),
                sunray_probability_estimator: ProbabilityEstimator::new()
            },
            orchestrator_communicator: OrchestratorCommunicator::new(
                OrchestratorLoggingSender::new(tx_orchestrator, id, 0), // Orchestrator has ID 0
                OrchestratorLoggingReceiver::new(rx_orchestrator, id, 0),
                id
            ),
            planets_communicator: PlanetsCommunicator::new(
                HashMap::from([(current_planet, PlanetLoggingSender::new(tx_current_planet, id, current_planet))]),
                PlanetLoggingReceiver::new(rx_planet, id, 1), // First planet has ID 1
                id
            )
        }
    }

    fn run(&mut self) -> Result<(), String> {
        self.wait_for_start()?;
        while {
            let msg = self.orchestrator_communicator.recv()?;
            self.handle_orchestrator_message(msg)?
        } {} // no do while :(
        info!("Explorer {} terminating gracefully.", self.id);
        Ok(())
    }
}

impl HardwareAcceleratedExplorer {
    fn wait_for_start(&mut self) -> Result<(), String> {
        let msg = self.orchestrator_communicator.recv()?;
        if !msg.is_start_explorer_ai() {
            return Err(format!("Expected explorer AI start message, got {:?}", msg));
        }
        self.orchestrator_communicator.send_start_ack()
    }

    /// Returns Ok(true) if the explorer should continue running
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<bool, String> {
        match msg {
            OrchestratorToExplorer::ResetExplorerAI => {
                // TODO Handle reset explorer AI
                self.orchestrator_communicator.send_reset_ack()?
            }
            OrchestratorToExplorer::StopExplorerAI => {
                // TODO Handle stop explorer AI
                self.orchestrator_communicator.send_stop_ack()?
            }
            OrchestratorToExplorer::KillExplorer => {
                self.orchestrator_communicator.send_kill_ack()?;
                return Ok(false);
            }
            OrchestratorToExplorer::CurrentPlanetRequest =>
                self.orchestrator_communicator.send_current_planet_ack(self.state.current_planet)?,
            OrchestratorToExplorer::SupportedResourceRequest => {
                let resources = self.planets_communicator.basic_resource_discovery(self.state.current_planet)?;
                self.orchestrator_communicator.send_supported_resources_ack(resources)?;
            }
            OrchestratorToExplorer::SupportedCombinationRequest => {
                let combinations = self.planets_communicator.combination_rules_discovery(self.state.current_planet)?;
                self.orchestrator_communicator.send_combination_rules_ack(combinations)?;
            }
            OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                let generated =
                    self.planets_communicator.generate_basic_resource(self.state.current_planet, to_generate)?;
                // TODO store generated resource in bag
                self.orchestrator_communicator
                    .send_generation_ack(generated.ok_or("Cannot create resource".to_string()).map(|_| ()))?;
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                // TODO send the basic resources too
                // let combination_result =
                // self.planets_communicator.combine_resources(self.current_planet_id, to_generate)?;
                // // TODO store generated resource in bag if Ok
                // self.orchestrator_communicator.send_combination_ack(
                //     combination_result.map(|_|()).map_err(|(e, _, _)| e)
                // )?;
            }
            OrchestratorToExplorer::BagContentRequest => {
                RoundExecutor::new(&mut self.planets_communicator, &self.orchestrator_communicator, &mut self.state)
                    .execute_round()?;
                // TODO send bag content
                self.orchestrator_communicator.send_bag_content_ack(self.state.bag.to_bag_content())?;
            }
            _ => return Err(format!("Unexpected message type: {:?}", msg))
        }
        Ok(true)
    }
}

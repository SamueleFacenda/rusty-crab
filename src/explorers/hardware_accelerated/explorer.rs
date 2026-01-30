use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::{BagContent, Explorer};
use crate::explorers::hardware_accelerated::logging_channel::{OrchestratorLoggingReceiver, OrchestratorLoggingSender, PlanetLoggingReceiver};
use crate::explorers::hardware_accelerated::orchestrator_communicator::OrchestratorCommunicator;
use crate::explorers::hardware_accelerated::planets_communicator::PlanetsCommunicator;

pub struct HardwareAcceleratedExplorer {
    id: ID,
    current_planet_id: ID,

    // Resources
    bag: Bag,

    // Information collection
    knowledge: ExplorerKnowledge,
    orchestrator_communicator: OrchestratorCommunicator,
    planets_communicator: PlanetsCommunicator,
}

struct Bag {
    // not implemented in example
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
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        HardwareAcceleratedExplorer {
            id,
            current_planet_id: current_planet,
            orchestrator_communicator: OrchestratorCommunicator::new(
                OrchestratorLoggingSender::new(tx_orchestrator, id),
                OrchestratorLoggingReceiver::new(rx_orchestrator, id, 0), // Orchestrator has ID 0
                id,
            ),
            planets_communicator: PlanetsCommunicator::new(
                PlanetLoggingReceiver::new(rx_planet, id, 1), // First planet has ID 1
                id
            ),
            bag: Bag {},
            knowledge: ExplorerKnowledge {},
        }
    }

    fn run(&mut self) -> Result<(), String> {
        self.wait_for_start()?;
        while {
            let msg = self.orchestrator_communicator.recv()?;
            self.handle_orchestrator_message(msg)?
        } {} // no do while :(
        Ok(())
    }
}

impl HardwareAcceleratedExplorer {
    fn wait_for_start(&mut self) -> Result<(), String> {
        let msg = self.orchestrator_communicator.recv()?;
        if !msg.is_start_explorer_ai() {
            return Err(format!("Expected explorer AI start message, got {:?}", msg));
        }
        Ok(())
    }


    /// Returns Ok(true) if the explorer should continue running
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<bool, String> {
        match msg {
            OrchestratorToExplorer::ResetExplorerAI => {
                // Handle reset explorer AI
                self.orchestrator_communicator.send_reset_ack()?
            }
            OrchestratorToExplorer::StopExplorerAI => {
                // Handle stop explorer AI
                self.orchestrator_communicator.send_stop_ack()?
            }
            OrchestratorToExplorer::KillExplorer => {
                self.orchestrator_communicator.send_kill_ack()?;
                return Ok(false);
            }
            OrchestratorToExplorer::CurrentPlanetRequest => {
                self.orchestrator_communicator.send_current_planet_ack(self.current_planet_id)?
            }
            OrchestratorToExplorer::SupportedResourceRequest => {
                let resources = self.planets_communicator.basic_resource_discovery(self.current_planet_id)?;
                self.orchestrator_communicator.send_supported_resources_ack(resources)?;
            }
            OrchestratorToExplorer::SupportedCombinationRequest => {
                let combinations = self.planets_communicator.combination_rules_discovery(self.current_planet_id)?;
                self.orchestrator_communicator.send_combination_rules_ack(combinations)?;
            }
            OrchestratorToExplorer::GenerateResourceRequest { to_generate } => {
                let generated = self.planets_communicator.generate_basic_resource(self.current_planet_id, to_generate)?;
                // TODO store generated resource in bag
                self.orchestrator_communicator.send_generation_ack(
                    generated.ok_or("Cannot create resource".to_string()).map(|_|())
                )?;
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                // TODO send the basic resources too
                // let combination_result = self.planets_communicator.combine_resources(self.current_planet_id, to_generate)?;
                // // TODO store generated resource in bag if Ok
                // self.orchestrator_communicator.send_combination_ack(
                //     combination_result.map(|_|()).map_err(|(e, _, _)| e)
                // )?;
            }

            OrchestratorToExplorer::BagContentRequest => {
                // TODO do the round choices
                // TODO send bag content
                self.orchestrator_communicator.send_bag_content_ack(BagContent{})?;
            }

            _ => return Err(format!("Unexpected message type: {:?}", msg)),
        }
        Ok(true)
    }
}

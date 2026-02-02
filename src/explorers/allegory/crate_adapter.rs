use crate::explorers::Explorer;
use crate::explorers::allegory::logging::emit_info;
use crate::explorers::explorer::BagContent;
use crate::explorers::allegory::explorer::{AllegoryExplorer, ExplorerMode};
use common_game::components::resource::ComplexResourceType::{Dolphin, Water};
use common_game::components::resource::{BasicResourceType, ResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use crate::explorers::allegory::explorer::ExplorerMode::Auto;

impl Explorer for AllegoryExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_first_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        let task = HashMap::from([
            (ResourceType::Complex(Dolphin), 2),
            (ResourceType::Complex(Water), 4),
            (ResourceType::Basic(BasicResourceType::Hydrogen), 4),
            (ResourceType::Basic(BasicResourceType::Oxygen), 2),
        ]);
        AllegoryExplorer {
            id,
            current_planet_id: current_planet,
            mode: ExplorerMode::Stopped,
            rx_orchestrator,
            tx_orchestrator, // Currently not fixable
            tx_planet: tx_first_planet,
            rx_planet,
            bag: Default::default(),
            bag_content: BagContent{content: HashMap::new()},
            knowledge: Default::default(),
            task,
            simple_resources_task: HashMap::new(),
        }
    }

    fn run(&mut self) -> Result<(), String> {
        // Await starting message
        loop {
            let start_message = self.rx_orchestrator.recv();
            match start_message {
                Ok(msg) => {
                    match msg {
                        OrchestratorToExplorer::StartExplorerAI => {
                            self.mode = Auto;
                            self.handle_orchestrator_message(msg)?;
                            break;
                        }
                        _ => {} // Discard anything else
                    }
                }
                _ => {} // Discard errors too before startup
            }
        }
        // Start real execution
        emit_info(self.id, format!("Started Allegory loop with ID {}", self.id));
        loop {
            // Exit condition
            match self.mode {
                ExplorerMode::Killed | ExplorerMode::Retired => break,
                _ => {}
            }
            
            // Check for orchestrator commands first
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    self.handle_orchestrator_message(msg)?;
                    // If killed or stopped, skip this turn
                    match self.mode {
                        ExplorerMode::Killed | ExplorerMode::Retired => break,
                        ExplorerMode::Stopped => continue,
                        _ => {}
                    }
                },
                Err(_) => break // Channel closed
            }
            
            // Run turn logic if in Auto mode
            if matches!(self.mode, ExplorerMode::Auto) {
                self.run_loop()?;
            }
        }
        // If retired or killed, terminate execution
        Ok(())
    }
}

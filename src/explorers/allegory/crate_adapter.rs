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
use crate::explorers::allegory::bag::Bag;
use crate::explorers::allegory::explorer::ExplorerMode::{Auto, Retired};
use crate::explorers::allegory::knowledge::ExplorerKnowledge;

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
            (ResourceType::Complex(Dolphin), 1),
            (ResourceType::Complex(Water), 1),
            (ResourceType::Basic(BasicResourceType::Hydrogen), 2),
            (ResourceType::Basic(BasicResourceType::Oxygen), 1),
        ]);

        let mut explorer = AllegoryExplorer {
            id,
            current_planet_id: current_planet,
            mode: ExplorerMode::Stopped,
            rx_orchestrator,
            tx_orchestrator,
            tx_planet: tx_first_planet,
            rx_planet,
            bag: Bag::default(),
            bag_content: BagContent{content: HashMap::new()},
            knowledge: ExplorerKnowledge::default(),
            task,
            simple_resources_task: HashMap::new(),
        };
        explorer.complex_to_simple_list();
        println!("{:?}", explorer.simple_resources_task);
        explorer
    }

    fn run(&mut self) -> Result<(), String> {
        // Await starting message
        loop {
            let start_message = self.rx_orchestrator.recv();
            if let Ok(msg) = start_message && let OrchestratorToExplorer::StartExplorerAI = msg {
                self.mode = Auto;
                self.handle_orchestrator_message(msg)?;
                break;
            } // Discard anything else (including errors) before startup
        }
        // Start real execution
        emit_info(self.id, format!("Started Allegory loop with ID {}", self.id));
        loop {
            // Exit condition
            match self.mode {
                ExplorerMode::Killed |  ExplorerMode::Retired => {
                    break;
                },
                _ => {}
            }
            if self.verify_win() {
                self.mode = Retired;
                break;
            }
            
            // Check for orchestrator commands first
            let mut messages = Vec::new();
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    messages.push(msg);
                    while let Ok(m) = self.rx_orchestrator.try_recv() {
                        messages.push(m);
                    }
                },
                Err(_) => break // Channel closed
            }

            // debug: priority Check for Kill 
            // kept getting [ERROR] Orchestrator terminated with error: Expected explorer 8 to respond with KillExplorerResult, but got BagContentResponse { explorer_id: 8, bag_content: BagContent { content: {} } }
            // If the orchestrator sent a Kill request,  prioritize it and ignore previous requests
            if messages.iter().any(|m| matches!(m, OrchestratorToExplorer::KillExplorer)) {
                 let kill_msg = messages.into_iter().find(|m| matches!(m, OrchestratorToExplorer::KillExplorer)).unwrap();
                 self.handle_orchestrator_message(kill_msg)?;
                 break; // Break the outer loop immediately
            }

            // Normal processing
            for msg in messages {
                self.handle_orchestrator_message(msg)?;
                if matches!(self.mode, ExplorerMode::Killed | ExplorerMode::Retired) {
                    break;
                }
            }

            if matches!(self.mode, ExplorerMode::Stopped) {
            continue;
            }
            
            // Run turn logic if in Auto mode
            if matches!(self.mode, ExplorerMode::Auto) {
                self.run_loop()?;
            }
        }
        // If retired or killed, terminate execution
        if matches!(self.mode, ExplorerMode::Retired) {
            emit_info(self.id, "Concluded execution: retired".to_string());
            return Ok(())
        } else if matches!(self.mode, ExplorerMode::Killed) {
            emit_info(self.id, "Concluded execution: killed".to_string());
            return Ok(())
        }  
        emit_info(self.id, "Concluded execution".to_string());
        Ok(())
    }
}

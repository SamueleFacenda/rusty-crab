use std::collections::HashMap;
use common_game::components::resource::ComplexResourceType::{Dolphin, Water};
use common_game::components::resource::{BasicResourceType, ResourceType};
use crate::explorers::Explorer;
use allegory::explorer::{AllegoryExplorer, ExplorerMode};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use allegory::explorer::ExplorerMode::Auto;
use crate::explorers::explorer::BagContent;
use crate::explorers::BagContent as BagContent2;

impl Explorer for AllegoryExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_first_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        let task = HashMap::from(
            [(ResourceType::Complex(Dolphin),2),
                (ResourceType::Complex(Water),4),
                (ResourceType::Basic(BasicResourceType::Hydrogen), 4),
                (ResourceType::Basic(BasicResourceType::Oxygen), 2),]
        );
        AllegoryExplorer{
            id,
            current_planet_id: 0, // Orchestrator as a placeholder for now
            mode: ExplorerMode::Stopped,
            rx_orchestrator,
            tx_orchestrator, // Currently not fixable
            tx_planet: tx_first_planet,
            rx_planet,
            bag: Default::default(),
            bag_content: Default::default(),
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
                            break;
                        }
                        _ => {} // Discard anything else
                    }
                }
                Err(_) => {}
            }
        }
        // Start real execution
        

        Ok(())
    }
    
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        todo!()
    }

    
}

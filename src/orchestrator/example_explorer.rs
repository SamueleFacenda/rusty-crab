use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::orchestrator::{BagContent, Explorer};

#[allow(dead_code)]
pub struct ExampleExplorer {
    id: ID,

    // Current State
    current_planet_id: ID,
    mode: ExplorerMode,

    // Communication Channels
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>, // Not sure about the type here
    tx_planet: Option<Sender<ExplorerToPlanet>>,
    rx_planet: Receiver<PlanetToExplorer>,

    // Resources
    bag: Bag,

    // Information collection
    knowledge: ExplorerKnowledge,
}

enum ExplorerMode {
    Auto,
    Manual,
    Stopped,
    Killed,
}

struct Bag {
    // not implemented in example
}
struct ExplorerKnowledge {
    // not implemented in example
}

impl Explorer for ExampleExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        ExampleExplorer {
            id,
            current_planet_id: current_planet,
            mode: ExplorerMode::Auto,
            rx_orchestrator,
            tx_orchestrator,
            tx_planet: None,
            rx_planet,
            bag: Bag {},
            knowledge: ExplorerKnowledge {},
        }
    }

    fn run(&mut self) -> Result<(), String> {
        loop {
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    if let Err(e) = self.handle_orchestrator_message(msg) {
                        log::error!("Error handling orchestrator message: {e}");
                    }
                }
                Err(e) => {
                    log::error!("Error receiving message from orchestrator: {e}");
                    Err(e.to_string())?;
                }
            }
        }
    }

    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        Ok(())
    }
}

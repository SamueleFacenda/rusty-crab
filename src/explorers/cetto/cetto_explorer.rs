use std::collections::{HashMap, HashSet};
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, GenericResource, Oxygen, ResourceType};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::{BagContent, Explorer};
use crate::explorers::cetto::knowledge::ExplorerKnowledge;

pub struct CettoExplorer {
    id: ID,

    // Current State
    current_planet_id: ID,
    mode: ExplorerMode,

    // Communication Channels
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
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
    basic_resources: Vec<BasicResource>,
    complex_resources: Vec<ComplexResource>,
}

impl Default for Bag {
    fn default() -> Self {
        Bag {
            basic_resources: vec![],
            complex_resources: vec![]
        }
    }
}



impl Explorer for CettoExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_current_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        CettoExplorer {
            id,
            current_planet_id: current_planet,
            mode: ExplorerMode::Auto,
            rx_orchestrator,
            tx_orchestrator,
            tx_planet: Some(tx_current_planet),
            rx_planet,
            bag: Bag::default(),
            knowledge: ExplorerKnowledge::default(),
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
}

impl CettoExplorer {
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        Ok(())
    }
}

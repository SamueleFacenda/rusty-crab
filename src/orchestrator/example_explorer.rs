use std::collections::HashMap;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

pub trait Explorer{
    fn handle_orchestrator_message(
        &mut self,
        msg: OrchestratorToExplorer,
    ) -> Result<(), String>;
}
pub struct ExampleExplorer {
    id: ID,

    // Current State
    current_planet_id: ID,
    mode: ExplorerMode,

    // Communication Channels
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<()>>, // Not sure about the type here
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
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        Ok(())
    }
}
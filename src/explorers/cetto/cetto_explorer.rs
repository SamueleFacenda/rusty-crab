use std::collections::{HashMap, HashSet};
use std::thread::current;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, GenericResource, Oxygen, ResourceType};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::{BagContent, Explorer};
use crate::explorers::cetto::explorator_channel::{OrchestratorLoggingReceiver, OrchestratorLoggingSender, PlanetLoggingReceiver, PlanetLoggingSender};
use crate::explorers::cetto::knowledge::ExplorerKnowledge;

pub struct CettoExplorer {
    id: ID,

    // Current State
    current_planet_id: ID,
    mode: ExplorerMode,

    // Communication Channels
    rx_orchestrator: OrchestratorLoggingReceiver,
    tx_orchestrator: OrchestratorLoggingSender,
    rx_planet: PlanetLoggingReceiver,
    tx_planets: HashMap<ID, PlanetLoggingSender>,  // To communicate with all planets

    // Resources
    bag: Bag,

    // Information collection
    knowledge: ExplorerKnowledge,
}

#[derive(PartialEq)]
enum ExplorerMode {
    Auto,
    Manual,
    Killed,
}

struct Bag {
    basic_resources: Vec<BasicResource>,
    complex_resources: Vec<ComplexResource>,
}

impl Default for Bag {
    fn default() -> Self {
        Bag {
            basic_resources: Vec::new(),
            complex_resources: Vec::new()
        }
    }
}



impl Explorer for CettoExplorer {
    fn new(
        id: ID,
        current_planet_id: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        rx_planet: Receiver<PlanetToExplorer>,
        tx_first_planet: Sender<ExplorerToPlanet>,
    ) -> Self {
        let mut tx_planets = HashMap::new();
        let tx_first_planet = PlanetLoggingSender::new(tx_first_planet, id, current_planet_id);
        tx_planets.insert(current_planet_id, tx_first_planet);
        CettoExplorer {
            id,
            current_planet_id,
            mode: ExplorerMode::Auto,
            rx_orchestrator: OrchestratorLoggingReceiver::new(rx_orchestrator, id, 0),
            tx_orchestrator: OrchestratorLoggingSender::new(tx_orchestrator, id, 0),
            rx_planet: PlanetLoggingReceiver::new(rx_planet, id, 1),
            tx_planets,
            bag: Bag::default(),
            knowledge: ExplorerKnowledge::default(),
        }
    }

    fn run(&mut self) -> Result<(), String> {
        loop {
            let orch_msg = self.rx_orchestrator.recv();

            match orch_msg {
                Ok(msg) => {
                    if let Err(e) = self.handle_orchestrator_message(msg) {
                        log::error!("Error handling orchestrator message: {e}");
                        return Err(e);
                    }
                },
                Err(e) => {
                    log::error!("Error receiving message from orchestrator: {e}");
                    return Err(e.to_string());
                }
            }
        }
    }
}

impl CettoExplorer {
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        if let Err(e) = self.handle_nonsense_requests(&msg) {
            return Err(e);
        }

        match msg {
            OrchestratorToExplorer::StartExplorerAI => {
                self.mode = ExplorerMode::Auto;
                self.tx_orchestrator.send(ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: self.id })
            },
            OrchestratorToExplorer::StopExplorerAI => {
                self.mode = ExplorerMode::Manual;
                self.tx_orchestrator.send(ExplorerToOrchestrator::StopExplorerAIResult { explorer_id: self.id })
            },
            OrchestratorToExplorer::ResetExplorerAI => {
                // Mode stays the same unless Killed
                if self.mode == ExplorerMode::Killed {
                    self.mode = ExplorerMode::Manual;
                }
                // Reset bag and knowledge
                self.bag = Bag::default();
                self.knowledge = ExplorerKnowledge::default();

                self.tx_orchestrator.send(ExplorerToOrchestrator::ResetExplorerAIResult { explorer_id: self.id })
            },
            OrchestratorToExplorer::KillExplorer => {
                self.mode = ExplorerMode::Killed;
                self.tx_orchestrator.send(ExplorerToOrchestrator::KillExplorerResult {explorer_id: self.id })
            },
            OrchestratorToExplorer::CurrentPlanetRequest => {
                self.tx_orchestrator.send(ExplorerToOrchestrator::CurrentPlanetResult {
                    explorer_id: self.id,
                    planet_id: self.current_planet_id
                })
            },
            OrchestratorToExplorer::SupportedResourceRequest => {
                // Ask the current planet, wait for response, and then respond to the orchestrator
                self.tx_planets[&self.current_planet_id].send(ExplorerToPlanet::SupportedResourceRequest { explorer_id: self.id })?;
                let planet_response = self.rx_planet.recv()
                    .map_err(|err| format!("Exception when waiting for planet response: {err}"))?;

                self.tx_orchestrator.send(
                    ExplorerToOrchestrator::SupportedResourceResult {
                        explorer_id: self.id,
                        supported_resources: planet_response.into_supported_resource_response().unwrap()
                    }
                )
            },
            OrchestratorToExplorer::SupportedCombinationRequest => {
                // Ask the current planet, wait for response, and then respond to the orchestrator
                self.tx_planets[&self.current_planet_id].send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: self.id })?;
                let planet_response = self.rx_planet.recv()
                    .map_err(|err| format!("Exception when waiting for planet response: {err}"))?;

                self.tx_orchestrator.send(
                    ExplorerToOrchestrator::SupportedCombinationResult {
                        explorer_id: self.id,
                        combination_list: planet_response.into_supported_combination_response().unwrap()
                    }
                )
            },
            OrchestratorToExplorer::GenerateResourceRequest { to_generate} => {
                // Ask the current planet, wait for response, save resource, respond to the orchestrator
                self.tx_planets[&self.current_planet_id].send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: self.id, resource: to_generate})?;
                let planet_response = self.rx_planet.recv()
                    .map_err(|err| format!("Exception when waiting for planet response: {err}"))?;

                // Add to the bag if produced
                let data = planet_response.into_generate_resource_response().unwrap();
                let explorer_response =
                    if let Some(res) = data {
                        self.bag.basic_resources.push(*res);
                        Ok(())
                    } else {
                        Err("The resource could not be generated".to_string())
                    };

                self.tx_orchestrator.send(
                    ExplorerToOrchestrator::GenerateResourceResponse {
                        explorer_id: self.id,
                        generated: explorer_response
                    }
                )
            },
            OrchestratorToExplorer::CombineResourceRequest { to_generate} => {

            },
            OrchestratorToExplorer::BagContentRequest => {

            },
            OrchestratorToExplorer::NeighborsResponse { neighbors} => {

            },
            OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id } => {

            },
        }
    }

    fn handle_nonsense_requests(&mut self, msg: &OrchestratorToExplorer) -> Result<(), String> {
        if self.mode == ExplorerMode::Killed && !matches!(msg, OrchestratorToExplorer::ResetExplorerAI) {
            return Err("Explorer is dead and is requested to do something".to_string());
        }
        // if self.mode == ExplorerMode::Auto && matches!(msg, OrchestratorToExplorer::StartExplorerAI){
        //     return Err("ExplorerAI is already running and is requested to start".to_string());
        // }
        // if self.mode == ExplorerMode::Manual && matches!(msg, OrchestratorToExplorer::StopExplorerAI) {
        //     return Err("ExplorerAI is already stopped and is requested to stop".to_string());
        // }
        Ok(())
    }
}

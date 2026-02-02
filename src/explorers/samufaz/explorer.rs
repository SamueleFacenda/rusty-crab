use std::collections::HashMap;

use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use log::info;

use super::{Bag, GalaxyKnowledge, OrchestratorCommunicator, OrchestratorLoggingReceiver, OrchestratorLoggingSender,
            PlanetLoggingReceiver, PlanetLoggingSender, PlanetsCommunicator, ProbabilityEstimator,
            get_resource_request};
use crate::explorers::samufaz::round_executor::RoundExecutor;
use crate::explorers::{BagContent, Explorer};

// DTO for the explorer's state
pub(super) struct ExplorerState {
    pub bag: Bag,
    pub knowledge: Option<GalaxyKnowledge>,
    pub current_planet: ID,
    pub asteroid_probability_estimator: ProbabilityEstimator,
    pub sunray_probability_estimator: ProbabilityEstimator
}

pub struct SamuFazExplorer {
    id: ID,
    stopped: bool,

    state: ExplorerState,

    orchestrator_communicator: OrchestratorCommunicator,
    planets_communicator: PlanetsCommunicator
}

impl Explorer for SamuFazExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_current_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>
    ) -> Self {
        SamuFazExplorer {
            id,
            stopped: false,
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

impl SamuFazExplorer {
    fn wait_for_start(&mut self) -> Result<(), String> {
        let msg = self.orchestrator_communicator.recv()?;
        if !msg.is_start_explorer_ai() {
            return Err(format!("Expected explorer AI start message, got {msg:?}"));
        }
        self.orchestrator_communicator.send_start_ack()
    }

    /// Returns Ok(true) if the explorer should continue running
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<bool, String> {
        match msg {
            OrchestratorToExplorer::ResetExplorerAI => {
                self.state = ExplorerState {
                    bag: Bag { res: HashMap::new() },
                    knowledge: None,
                    current_planet: self.state.current_planet,
                    asteroid_probability_estimator: ProbabilityEstimator::new(),
                    sunray_probability_estimator: ProbabilityEstimator::new()
                };
                self.orchestrator_communicator.send_reset_ack()?;
            }
            OrchestratorToExplorer::StopExplorerAI => {
                self.stopped = true;
                self.orchestrator_communicator.send_stop_ack()?;
            }
            OrchestratorToExplorer::StartExplorerAI => {
                self.stopped = false;
                self.orchestrator_communicator.send_start_ack()?;
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
                let mut generated =
                    self.planets_communicator.generate_basic_resource(self.state.current_planet, to_generate)?;

                self.orchestrator_communicator
                    .send_generation_ack(generated.as_ref().map(|_| ()).ok_or("Cannot create resource".to_string()))?;

                if let Some(resource) = generated.take() {
                    self.state.bag.insert_basic(resource);
                }
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                let ingredients = self.state.bag.get_recipe_ingredients(to_generate);
                if ingredients.is_none() {
                    self.orchestrator_communicator
                        .send_combination_ack(Err("Not enough resources to combine".to_string()))?;
                    return Ok(true);
                }
                let (a, b) = ingredients.unwrap();
                let req = get_resource_request(to_generate, a, b);

                match self.planets_communicator.combine_resources(self.state.current_planet, req)? {
                    Ok(res) => {
                        // Store generated resource in bag
                        self.state.bag.insert_complex(res);
                        self.orchestrator_communicator.send_combination_ack(Ok(()))?;
                    }
                    Err((e, a, b)) => {
                        // Store the unused resources back in the bag
                        self.state.bag.res.entry(a.get_type()).or_default().push(a);
                        self.state.bag.res.entry(b.get_type()).or_default().push(b);
                        self.orchestrator_communicator.send_combination_ack(Err(e))?;
                    }
                }
            }
            OrchestratorToExplorer::BagContentRequest => {
                if !self.stopped {
                    // Stop the autonomous decision-making if the explorer is stopped
                    RoundExecutor::new(
                        &mut self.planets_communicator,
                        &self.orchestrator_communicator,
                        &mut self.state
                    )
                    .execute_round()?;
                }
                self.orchestrator_communicator.send_bag_content_ack(self.state.bag.to_bag_content())?;
            }
            OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet: Some(sender), planet_id } => {
                self.planets_communicator.add_planet(planet_id, sender);
                self.planets_communicator.set_current_planet(planet_id);
                self.state.current_planet = planet_id;
                self.orchestrator_communicator.send_move_ack(planet_id)?;
            }
            _ => return Err(format!("Unexpected message type: {msg:?}"))
        }
        Ok(true)
    }
}

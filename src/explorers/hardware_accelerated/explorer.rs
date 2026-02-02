use std::collections::HashMap;

use bevy_tweening::AnimTargetKind::Resource;
use common_game::components::resource::BasicResource::Carbon;
use common_game::components::resource::{BasicResourceType, ComplexResourceRequest, ComplexResourceType,
                                        GenericResource, ResourceType};
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
                *content.content.entry(resource.get_type()).or_default() += 1;
            }
        }
        content
    }
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
                let mut generated =
                    self.planets_communicator.generate_basic_resource(self.state.current_planet, to_generate)?;

                self.orchestrator_communicator
                    .send_generation_ack(generated.as_ref().map(|_| ()).ok_or("Cannot create resource".to_string()))?;

                if let Some(resource) = generated.take() {
                    self.state
                        .bag
                        .res
                        .entry(ResourceType::Basic(resource.get_type()))
                        .or_default()
                        .push(GenericResource::BasicResources(resource));
                }
            }
            OrchestratorToExplorer::CombineResourceRequest { to_generate } => {
                let (a, b) = get_resource_recipe(&to_generate);
                let res_a = self.state.bag.res.entry(a).or_default();
                if res_a.is_empty() {
                    // Not enough resources to combine
                    self.orchestrator_communicator
                        .send_combination_ack(Err("Not enough resources to combine".to_string()))?;
                    return Ok(true);
                }
                let a_resource = res_a.pop().unwrap();
                let res_b = self.state.bag.res.entry(b).or_default();
                if res_b.is_empty() {
                    // Not enough resources to combine, put back a_resource
                    self.state.bag.res.entry(a).or_default().push(a_resource);
                    self.orchestrator_communicator
                        .send_combination_ack(Err("Not enough resources to combine".to_string()))?;
                    return Ok(true);
                }
                let b_resource = res_b.pop().unwrap();

                let req = get_resource_request(to_generate, a_resource, b_resource);

                match self.planets_communicator.combine_resources(self.state.current_planet, req)? {
                    Ok(res) => {
                        // Store generated resource in bag
                        self.state
                            .bag
                            .res
                            .entry(ResourceType::Complex(res.get_type()))
                            .or_default()
                            .push(GenericResource::ComplexResources(res));
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
                RoundExecutor::new(&mut self.planets_communicator, &self.orchestrator_communicator, &mut self.state)
                    .execute_round()?;
                self.orchestrator_communicator.send_bag_content_ack(self.state.bag.to_bag_content())?;
            }
            OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet: Some(sender), planet_id } => {
                self.planets_communicator.add_planet(planet_id, sender);
                self.planets_communicator.set_current_planet(planet_id);
                self.state.current_planet = planet_id;
                self.orchestrator_communicator.send_move_ack(planet_id)?;
            }
            _ => return Err(format!("Unexpected message type: {:?}", msg))
        }
        Ok(true)
    }
}

fn get_resource_recipe(resource: &ComplexResourceType) -> (ResourceType, ResourceType) {
    match resource {
        ComplexResourceType::Water =>
            (ResourceType::Basic(BasicResourceType::Hydrogen), ResourceType::Basic(BasicResourceType::Oxygen)),
        ComplexResourceType::Diamond =>
            (ResourceType::Basic(BasicResourceType::Carbon), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Life =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Basic(BasicResourceType::Carbon)),
        ComplexResourceType::Robot =>
            (ResourceType::Basic(BasicResourceType::Silicon), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::Dolphin =>
            (ResourceType::Complex(ComplexResourceType::Water), ResourceType::Complex(ComplexResourceType::Life)),
        ComplexResourceType::AIPartner =>
            (ResourceType::Complex(ComplexResourceType::Robot), ResourceType::Complex(ComplexResourceType::Diamond)),
    }
}

fn get_resource_request(
    res_type: ComplexResourceType,
    a: GenericResource,
    b: GenericResource
) -> ComplexResourceRequest {
    match res_type {
        ComplexResourceType::Water => ComplexResourceRequest::Water(a.to_hydrogen().unwrap(), b.to_oxygen().unwrap()),
        ComplexResourceType::Diamond => ComplexResourceRequest::Diamond(a.to_carbon().unwrap(), b.to_carbon().unwrap()),
        ComplexResourceType::Life => ComplexResourceRequest::Life(a.to_water().unwrap(), b.to_carbon().unwrap()),
        ComplexResourceType::Robot => ComplexResourceRequest::Robot(a.to_silicon().unwrap(), b.to_life().unwrap()),
        ComplexResourceType::Dolphin => ComplexResourceRequest::Dolphin(a.to_water().unwrap(), b.to_life().unwrap()),
        ComplexResourceType::AIPartner =>
            ComplexResourceRequest::AIPartner(a.to_robot().unwrap(), b.to_diamond().unwrap()),
    }
}

use std::collections::{HashMap, HashSet};
use std::thread::current;
use bevy::ecs::system::IntoResult;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType, GenericResource, Oxygen, ResourceType};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::{BagContent, Explorer};
use crate::explorers::cetto::communication::{OrchestratorLoggingReceiver, OrchestratorLoggingSender, PlanetLoggingReceiver, PlanetLoggingSender};
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

impl Bag {
    fn get_basic(&mut self, target_type: BasicResourceType) -> Option<BasicResource> {
        if let Some(index) = self.basic_resources.iter()
            .position(|res| res.get_type() == target_type){
            return Some(self.basic_resources.remove(index))
        }
        None
    }

    fn get_complex(&mut self, target_type: ComplexResourceType) -> Option<ComplexResource> {
        if let Some(index) = self.complex_resources.iter()
            .position(|res| res.get_type() == target_type){
            return Some(self.complex_resources.remove(index))
        }
        None
    }

    fn push_generic(&mut self, resource: GenericResource) {
        match resource {
            GenericResource::BasicResources(value) => {
                self.basic_resources.push(value);
            },
            GenericResource::ComplexResources(value ) => {
                self.complex_resources.push(value);
            }
        }
    }

    fn generate_bag_content_hashmap(&self) -> HashMap<ResourceType, usize> {
        let mut output: HashMap<ResourceType, usize> = HashMap::new();

        for basic in self.basic_resources {
            *output.entry(ResourceType::Basic(basic.get_type())).or_insert(0) += 1;
        }
        for complex in self.complex_resources {
            *output.entry(ResourceType::Complex(complex.get_type())).or_insert(0) += 1;
        }
        output
    }

    fn has_basic(&self, typ: BasicResourceType, target_count: i32) -> bool {
        let mut count = 0;
        for el in self.basic_resources {
            if el.get_type() == typ {
                count += 1;
            }
        }
        count >= target_count
    }

    fn has_complex(&self, typ: ComplexResourceType, target_count: i32) -> bool {
        let mut count = 0;
        for el in self.complex_resources {
            if el.get_type() == typ {
                count += 1;
            }
        }
        count >= target_count
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
                    // If the message is ok but the explorer's actions create errors
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
            OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id } => {
                self.handle_move_planet(sender_to_new_planet, planet_id)
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
                self.handle_generate_resource_request(to_generate)?;

                self.tx_orchestrator.send(
                    ExplorerToOrchestrator::GenerateResourceResponse {
                        explorer_id: self.id,
                        generated: Ok(())
                    }
                )
            },
            OrchestratorToExplorer::CombineResourceRequest { to_generate} => {
                self.handle_combine_resource_request(to_generate)?;
                // Respond to Orchestrator
                self.tx_orchestrator.send(ExplorerToOrchestrator::CombineResourceResponse {explorer_id: self.id, generated: Ok(()) })
            },
            OrchestratorToExplorer::BagContentRequest => {
                // Play the turn and then respond
                self.play_turn();

                // Respond
                let content = BagContent {
                    content: self.bag.generate_bag_content_hashmap()
                };
                self.tx_orchestrator.send(
                    ExplorerToOrchestrator::BagContentResponse {
                        explorer_id: self.id,
                        bag_content: content
                    }
                )
            },
            OrchestratorToExplorer::NeighborsResponse { neighbors} => {
                // Update galaxyKnowledge
                for neighbor_id in neighbors {
                    self.knowledge.add_bi_connection(self.current_planet_id, neighbor_id);
                }
                Ok(())
            },
        }
    }






    fn play_turn(&mut self) -> Result<(), String> {
        if self.mode == ExplorerMode::Manual || self.knowledge.goal_completed() {
            return Ok(());
        }

        // CASE 2: Visit all reachable planets and exploit them

        let mut visited = HashSet::new();
        visited.insert(self.current_planet_id);
        self.exploit_current_planet()?;



        // self.move_safest_planet();
        Ok(())
    }

    fn request_neighbors(&mut self) -> Result<(), String> {
        self.tx_orchestrator.send(
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id: self.id,
                current_planet_id: self.current_planet_id
            }
        )?;
        let orch_response = self
            .rx_orchestrator
            .recv()
            .map_err(|err| format!("Exception when waiting for orchestrator response: {err}"))?;
        self.handle_orchestrator_message(orch_response)
    }

    fn move_explorer(&mut self, dst_planet_id: ID) -> Result<(), String> {
        self.tx_orchestrator.send(
            ExplorerToOrchestrator::TravelToPlanetRequest {
                explorer_id: self.id,
                current_planet_id: self.current_planet_id,
                dst_planet_id
            }
        )?;
        let orch_response = self
            .rx_orchestrator
            .recv()
            .map_err(|err| format!("Exception when waiting for orchestrator response: {err}"))?;
        self.handle_orchestrator_message(orch_response)
    }

    fn exploit_current_planet(&mut self) -> Result<(), String>{
        let mut cells = self.get_available_cells()?;
        if cells == 0 { return Ok(()); }

        // Create complex resources if possible and needed
        let complex = self.get_planet_complex_types()?;
        for comp in complex {
            if self.can_combine(comp, cells) {
                self.handle_combine_resource_request(comp)?;
                cells -= 1;
            }
        }
        if cells == 0 { return Ok(()); }  // Again, to possibly lower the number of comms

        // Obtain basic resources if possible and needed
        let basic = self.get_planet_basic_types()?;
        for bas in basic {
            if !self.knowledge.basic_goal_completed(bas) && cells > 0 {
                self.handle_generate_resource_request(bas)?;
                cells -= 1;
            }
        }
        Ok(())
    }

    fn can_combine(&self, comp: ComplexResourceType, cells: u32) -> bool {
        !self.knowledge.complex_goal_completed(comp) && self.has_ingredients(comp) && cells > 0
    }

    fn has_ingredients(&self, typ: ComplexResourceType) -> bool {
        match typ {
            ComplexResourceType::Water => {
                self.bag.has_basic(BasicResourceType::Oxygen, 1) && self.bag.has_basic(BasicResourceType::Hydrogen, 1)
            },
            ComplexResourceType::Diamond => {
                self.bag.has_basic(BasicResourceType::Carbon, 2)
            },
            ComplexResourceType::Life => {
                self.bag.has_complex(ComplexResourceType::Water, 1) && self.bag.has_basic(BasicResourceType::Carbon, 1)
            },
            ComplexResourceType::Robot => {
                self.bag.has_complex(ComplexResourceType::Life, 1) && self.bag.has_basic(BasicResourceType::Silicon, 1)
            },
            ComplexResourceType::Dolphin => {
                self.bag.has_complex(ComplexResourceType::Water, 1) && self.bag.has_complex(ComplexResourceType::Life, 1)
            },
            ComplexResourceType::AIPartner => {
                self.bag.has_complex(ComplexResourceType::Robot, 1) && self.bag.has_complex(ComplexResourceType::Diamond, 1)
            }
        }
    }

    fn get_planet_basic_types(&self) -> Result<HashSet<BasicResourceType>, String> {
        self.tx_planets[&self.current_planet_id].send(
            ExplorerToPlanet::SupportedResourceRequest { explorer_id: self.id }
        )?;
        let list = self
            .rx_planet
            .recv()
            .map_err(|err| format!("Exception when waiting for planet response: {err}"))?
            .into_supported_resource_response()
            .unwrap();  // Safe because we know the type
        Ok(list)
    }

    fn get_planet_complex_types(&self) -> Result<HashSet<ComplexResourceType>, String> {
        self.tx_planets[&self.current_planet_id].send(
            ExplorerToPlanet::SupportedCombinationRequest { explorer_id: self.id }
        )?;
        let list = self
            .rx_planet
            .recv()
            .map_err(|err| format!("Exception when waiting for planet response: {err}"))?
            .into_supported_combination_response()
            .unwrap();  // Safe because we know the type
        Ok(list)
    }

    fn get_available_cells(&self) -> Result<u32, String> {
        self.tx_planets[&self.current_planet_id].send(
            ExplorerToPlanet::AvailableEnergyCellRequest { explorer_id: self.id }
        )?;
        let cells = self
            .rx_planet
            .recv()
            .map_err(|err| format!("Exception when waiting for planet response: {err}"))?
            .into_available_energy_cell_response()
            .unwrap();  // Safe because we know the type
        Ok(cells)
    }





    fn handle_move_planet(&mut self, sender_to_new_planet: Option<Sender<ExplorerToPlanet>>, planet_id: ID) -> Result<(), String> {
        // Update current_planet_id and its tx_planet, then respond.
        // If the new sender is None, stay on the same planet
        if let Some(tx_new_planet) = sender_to_new_planet {
            self.current_planet_id = planet_id;
            if !self.tx_planets.contains_key(&planet_id) {
                self.tx_planets.insert(planet_id, PlanetLoggingSender::new(tx_new_planet, self.id, planet_id));
            }
        }
        // Do nothing if the sender is None, just respond to the orch

        self.tx_orchestrator.send(
            ExplorerToOrchestrator::MovedToPlanetResult { explorer_id: self.id, planet_id: self.current_planet_id }
        )
    }

    fn handle_combine_resource_request(&mut self, to_generate: ComplexResourceType) -> Result<(), String> {
        let comp_res_req = match to_generate {
            ComplexResourceType::Water => { self.make_water_request()? },
            ComplexResourceType::Diamond => {self.make_diamond_request()? },
            ComplexResourceType::Life => {self.make_life_request()? },
            ComplexResourceType::Robot => {self.make_robot_request()? },
            ComplexResourceType::Dolphin => {self.make_dolphin_request()? },
            ComplexResourceType::AIPartner => {self.make_aipartner_request()? },
        };
        // Send request to Planet
        self.tx_planets[&self.current_planet_id].send(ExplorerToPlanet::CombineResourceRequest {
            explorer_id: self.id,
            msg: comp_res_req
        })?;

        // Process response from Planet
        let planet_response = self.rx_planet.recv()
            .map_err(|err| format!("Exception when waiting for planet response: {err}"))?
            .into_combine_resource_response()
            .unwrap(); // It's safe because we know the type


        match planet_response {
            Ok(successful_response) => {
                self.knowledge.decrease_from_goal(ResourceType::Complex(successful_response.get_type()));
                self.bag.complex_resources.push(successful_response);
            },
            Err((err, res1, res2)) => {
                // Add old resources to the bag, and return error
                self.bag.push_generic(res1);
                self.bag.push_generic(res2);
                return Err(format!("The combination has failed: {err}"))
            }
        }
        Ok(())
    }

    fn handle_generate_resource_request(&mut self, to_generate: BasicResourceType) -> Result<(), String> {
        self.tx_planets[&self.current_planet_id].send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: self.id, resource: to_generate})?;
        let planet_response = self.rx_planet.recv()
            .map_err(|err| format!("Exception when waiting for planet response: {err}"))?;

        // Add to the bag if produced
        let data = planet_response.into_generate_resource_response().unwrap();
        let explorer_response =
            if let Some(res) = data {
                self.knowledge.decrease_from_goal(ResourceType::Basic(res.get_type()));
                self.bag.basic_resources.push(*res);

                Ok(())
            } else {
                Err("The resource could not be generated".to_string())
            };
        explorer_response
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

    fn make_water_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_basic(BasicResourceType::Hydrogen);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_basic(BasicResourceType::Oxygen);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.basic_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::Water(res1.unwrap().to_hydrogen()?, res2.unwrap().to_oxygen()?))
    }

    fn make_diamond_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_basic(BasicResourceType::Carbon);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_basic(BasicResourceType::Carbon);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.basic_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::Diamond(res1.unwrap().to_carbon()?, res2.unwrap().to_carbon()?))
    }

    fn make_life_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_complex(ComplexResourceType::Water);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_basic(BasicResourceType::Carbon);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.complex_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::Life(res1.unwrap().to_water()?, res2.unwrap().to_carbon()?))
    }

    fn make_robot_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_basic(BasicResourceType::Silicon);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_complex(ComplexResourceType::Life);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.basic_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::Robot(res1.unwrap().to_silicon()?, res2.unwrap().to_life()?))
    }

    fn make_dolphin_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_complex(ComplexResourceType::Water);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_complex(ComplexResourceType::Life);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.complex_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::Dolphin(res1.unwrap().to_water()?, res2.unwrap().to_life()?))
    }

    fn make_aipartner_request(&mut self) -> Result<ComplexResourceRequest, String>{
        let res1 = self.bag.get_complex(ComplexResourceType::Robot);
        if res1.is_none() {
            return Err("Combination not possible, missing ingredient".to_string());
        }
        let res2 = self.bag.get_complex(ComplexResourceType::Diamond);
        if res2.is_none() {
            // First resource is Some, put it back in the Bag
            self.bag.complex_resources.push(res1.unwrap());
            return Err("Combination not possible, missing ingredient".to_string());
        }
        Ok(ComplexResourceRequest::AIPartner(res1.unwrap().to_robot()?, res2.unwrap().to_diamond()?))
    }
}

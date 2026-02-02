use common_game::components::resource::ResourceType;
use crate::explorers::BagContent;
use crate::explorers::allegory::{knowledge::StrategyState::*, logging::emit_warning};
use common_game::protocols::{
    orchestrator_explorer::{OrchestratorToExplorer},
    planet_explorer::PlanetToExplorer,
};
use crossbeam_channel::select;
use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator::BagContentResponse;
use crate::explorers::allegory::explorer::AllegoryExplorer;
use crate::explorers::allegory::logging::{emit_error, emit_info};

impl AllegoryExplorer {
    /// Function to execute a loop. Begins with galaxy exploration, then performs its algorithm.
    pub fn run_loop(&mut self) -> Result<(), String> {
        self.explore();
        // self.trivial_collecting()?;
        // self.trivial_crafting()?;
        self.perform_next_step();
        self.go_to_safe_planet();
        self.conclude_turn()?;
        Ok(())
    }


    fn explore(&mut self) {
        self.knowledge.wipe_planets();
        let mut explored_planets = 0;
        // For each planet:
        loop {
            explored_planets += 1;
            // 1. Query 
            self.query_planet();
            // Wait for neighbors answer
            loop {
                select! {
                    recv(self.rx_orchestrator) -> msg => {
                        match msg {
                            Ok(m) => {
                                let is_neighbors_response = matches!(&m, OrchestratorToExplorer::NeighborsResponse { .. });
                                if let Err(e) = self.handle_orchestrator_message(m) {
                                    log::error!("Error handling orchestrator message: {}", e);
                                }
                                if is_neighbors_response {
                                    break;
                                }
                                if matches!(self.mode, crate::explorers::allegory::explorer::ExplorerMode::Killed | crate::explorers::allegory::explorer::ExplorerMode::Retired) {
                                    return;
                                }
                            },
                            Err(e) => {
                                log::error!("Orchestrator channel closed: {}", e);
                                return;
                            }
                        }
                    },
                    recv(self.rx_planet) -> msg => {
                        match msg {
                            Ok(m) => {
                                if let Err(e) = self.handle_planet_message(m) {
                                    log::error!("Error handling planet message: {}", e);
                                }
                            },
                            Err(e) => {
                                log::error!("Planet channel closed: {}", e);
                                return;
                            }
                        }
                    }
                }
            }

            // 2. Exit check
            let unexplored = self.knowledge.get_unexplored_planets();
            if explored_planets >= 7 {
                emit_info(self.id, "Explored all planets".to_string());
                break;
            }
            if unexplored.is_empty(){
                emit_info(self.id, "Finished exploring the solar system".to_string());
            }

            // 3. Move to next 
            match self.find_first_unexplored(&unexplored) {
                None => {
                    // Should not happen from previous condition but break
                    // emit_info(self.id, "Reached a supposedly unreachable condition".to_string());
                    break; 
                }, 
                Some(id) => {
                    match self.move_to_planet(id) {
                        Ok(_) => {},
                        Err(e) => emit_warning(self.id, format!("Failed to move to planet {}: {}", id, e)),
                    }
                }
            }
        }
    }

    /// Explorer AI: decides what to do next
    /// - Collecting: Moves to planets with resources and gathers them.
    /// - Crafting: Moves to planets with combinations and crafts complex resources.
    fn perform_next_step(&mut self) {
        match self.knowledge.get_current_state() {
            Collecting => {
                if let Err(e) = self.trivial_collecting() {
                    emit_warning(self.id, format!("Recoverable error in collecting strategy: {}", e));
                }
            },
            Crafting => {
                if let Err(e) = self.trivial_crafting() {
                    emit_warning(self.id, format!("Recoverable error in crafting strategy: {}", e));
                }
            },
            Finished => {},
            Failed => {},
        }
        if self.anything_left_on_the_shopping_list().is_none() {
            emit_info(self.id, "Collected all basic resources".to_string());
            self.change_state(Crafting);
        }
        
    }

    // Used trivial instead
    /*
    fn execute_collecting(&mut self) -> Result<(), String> {
        
        // check requirements
        let mut shopping_list = match self.anything_left_on_the_shopping_list() {
            Some(l) => l,
            None => {
                self.change_state(Crafting);
                return Ok(());
            }
        };

        // Find useful planets
        let useful_planets: Vec<common_game::utils::ID> = self.knowledge.planets.iter()
            .filter(|p| {
                p.get_latest_cells_number() > 0 && 
                shopping_list.keys().any(|res| p.get_resource_type().contains(res))
            })
            .map(|p| p.get_id())
            .collect();

        if useful_planets.is_empty() {
             emit_info(self.id, "No useful planets found for collecting.".to_string());
             // Return Ok to retry next turn or maybe switch mode? 
             // Logic suggests if we can't collect, we might be stuck, but existing logic didn't fail hard immediately.
             return Ok(());
        }

        // Visit planets
        for planet_id in useful_planets {
            // Check alive status since you apparently can't do that enough
            if matches!(self.mode, crate::explorers::allegory::explorer::ExplorerMode::Killed | crate::explorers::allegory::explorer::ExplorerMode::Retired) {
                return Ok(());
            }

            // Re-evaluate shopping list (optimization: skip if we got everything)
            shopping_list = match self.anything_left_on_the_shopping_list() {
                Some(l) => l,
                None => {
                    self.change_state(Crafting);
                    return Ok(());
                }
            };
            
            // Travel to planet
            self.move_to_planet(planet_id)?;
            
            // If we arrived
            if self.current_planet_id == planet_id {
                 // Determine what to collect
                let planet_resources_opt = self.knowledge.get_planet_knowledge(self.current_planet_id)
                    .map(|pk| pk.get_resource_type().clone());
                
                if let Some(planet_resources) = planet_resources_opt {
                    let resource_to_collect = shopping_list.keys()
                        .find(|r| planet_resources.contains(r));
                    
                    if let Some(res) = resource_to_collect {
                        // Attempt extraction ONCE
                        match self.gather_resource(*res) {
                            Ok(success) => {
                                if success {
                                    self.knowledge.consume_energy_cell(self.current_planet_id);
                                } else {
                                    // Mark empty if failed
                                    if let Some(pk) = self.knowledge.planets.iter_mut().find(|p| p.get_id() == self.current_planet_id) {
                                         pk.set_latest_cells_number(0);
                                    }
                                }
                            }
                            Err(e) => emit_warning(self.id, format!("Error gathering resource: {}", e)),
                        }
                    }
                }
            }
        }
        
        // Final check
        if self.anything_left_on_the_shopping_list().is_none() {
            self.change_state(Crafting);
        }
        
        Ok(())
    }
    */

    /// Easier version of collecting since the other one does not work
    fn trivial_collecting(&mut self) -> Result<(), String> {
        let bag_content = BagContent::from_bag(&self.bag);
        
        for resource in self.simple_resources_task.clone(){
            // Check if we already have enough of this resource
            let owned_count = bag_content
                .content
                .get(&ResourceType::Basic(resource.0))
                .copied()
                .unwrap_or(0);
            
            if owned_count >= resource.1 {
                continue; // Skip if we have enough
            }
            
            match self.find_best_planet_for_resource(resource.0){
                Some(planet) => {
                    self.move_to_planet(planet)?;
                    if self.knowledge.get_planet_knowledge(self.current_planet_id).unwrap().get_latest_cells_number() <= 1 {
                        continue;
                    }
                    self.request_resource_generation(resource.0);
                }
                None => continue,
            }
        }
        Ok(())
    }

    fn trivial_crafting(&mut self) -> Result<(), String> {
        for resource in self.task.clone() {
            match resource.0 {
                ResourceType::Basic(_) => {}
                ResourceType::Complex(complex_res) => {
                    // Find best planet for this combination
                    match self.find_best_planet_for_combination(complex_res) {
                        Some(planet) => {
                            self.move_to_planet(planet)?;
                            if self.knowledge.get_planet_knowledge(self.current_planet_id).unwrap().get_latest_cells_number() <= 1 {
                                continue;
                            }
                            // Attempt to combine the resource
                            if let Err(e) = self.combine_resource(complex_res) {
                                emit_warning(self.id, format!("Error combining resource: {}", e));
                            }
                        }
                        None => continue,
                    }
                }
            }
        }
        Ok(())
    }

    // Used trivial instead
    /* 
    fn execute_crafting(&mut self) -> Result<(), String> {
        if self.verify_win() {
            self.change_state(Finished);
            return Ok(());
        }

        match self.anything_left_on_the_crafting_list() {
            None => {
                // If crafting list empty but not win, maybe we missed something? Back to collecting?
                if self.anything_left_on_the_shopping_list().is_some() {
                    self.change_state(StrategyState::Collecting);
                }
                Ok(())
            },
            Some(list) => {
                // Check if current planet can craft something we need
                let current_combinations = self.knowledge
                    .get_planet_knowledge(self.current_planet_id)
                    .map(|pk| pk.get_combinations().clone())
                    .unwrap_or_default();
                
                let craftable_here = list.iter().find(|(res, _)| current_combinations.contains(res));

                if let Some((&res, _)) = craftable_here {
                    // Craft it
                    self.combine_resource(res)
                } else {
                    // Move
                    if let Some((&res, _)) = list.iter().max_by_key(|&(_, count)| count) {
                        let target = self.find_best_planet_for_combination(res);
                        if let Some(t) = target {
                            self.knowledge.set_destination(Some(t));
                            let next_hop = self.knowledge.get_next_hop(self.current_planet_id);
                            self.move_to_planet(next_hop)
                        } else {
                            Err("No planet found with required combination".to_string())
                        }
                    } else {
                        Ok(())
                    }
                }
            }
        }
    }
    */



    // Blocking Helpers
    fn gather_resource(&mut self, res: common_game::components::resource::BasicResourceType) -> Result<bool, String> {
        self.request_resource_generation(res);

        // Some planets time out and cause full orchestrator crash, trying to avoid that
        let timeout = crossbeam_channel::after(std::time::Duration::from_millis(500));
        
        // Wait for response
        loop {
            select! {
                recv(self.rx_planet) -> msg => {
                    match msg {
                        Ok(m) => {
                            let (is_response, success) = match &m {
                                PlanetToExplorer::GenerateResourceResponse { resource } => (true, resource.is_some()),
                                _ => (false, false),
                            };
                            
                            if let Err(e) = self.handle_planet_message(m) {
                                emit_error(self.id, format!("Error handling planet message: {}", e));
                            }
                            if is_response { return Ok(success); }
                        },
                        Err(e) => return Err(format!("Planet channel closed: {}", e)),
                    }
                },
                recv(self.rx_orchestrator) -> msg => {
                    match msg {
                        Ok(m) => {
                             // Handle interruptions
                             self.handle_orchestrator_message(m)?;
                             if matches!(self.mode, crate::explorers::allegory::explorer::ExplorerMode::Killed | crate::explorers::allegory::explorer::ExplorerMode::Retired) {
                                 return Ok(false);
                             }
                        },
                        Err(e) => return Err(format!("Orchestrator channel closed: {}", e)),
                    }
                },
                recv(timeout) -> _ => {
                    emit_warning(self.id, "Timeout waiting for planet resource generation".to_string());
                    return Ok(false);
                }
            }
        }
    }

    fn combine_resource(&mut self, res: common_game::components::resource::ComplexResourceType) -> Result<(), String> {
         let request = self.create_complex_request(res).ok_or("Failed to create request")?;
         if let Err(e) = self.request_resource_combination(request) {
             return Err(e);
         }
         
         let timeout = crossbeam_channel::after(std::time::Duration::from_millis(500));

        // Wait for response
        loop {
            select! {
                recv(self.rx_planet) -> msg => {
                     match msg {
                        Ok(m) => {
                            let is_response = matches!(m, PlanetToExplorer::CombineResourceResponse{..});
                            if let Err(e) = self.handle_planet_message(m) {
                                log::error!("Error handling planet message: {}", e);
                            }
                            if is_response { return Ok(()); }
                        },
                        Err(e) => return Err(format!("Planet channel closed: {}", e)),
                    }
                },
                recv(self.rx_orchestrator) -> msg => {
                     match msg {
                        Ok(m) => {
                            self.handle_orchestrator_message(m)?;
                            if matches!(self.mode, crate::explorers::allegory::explorer::ExplorerMode::Killed | crate::explorers::allegory::explorer::ExplorerMode::Retired) {
                                return Ok(());
                            }
                        },
                        Err(e) => return Err(format!("Orchestrator channel closed: {}", e)),
                    }
                },
                recv(timeout) -> _ => {
                    emit_warning(self.id, "Timeout waiting for planet resource combination".to_string());
                    return Err("Timeout waiting for planet resource combination".to_string());
                }
            }
        }
    }
    
    // Helper search functions
    /// Looks for the planet with the most energy cells producing a given resource
    fn find_best_planet_for_resource(&self, res: common_game::components::resource::BasicResourceType) -> Option<common_game::utils::ID> {
        self.knowledge.planets.iter()
            .filter(|p| p.get_resource_type().contains(&res))
            .max_by_key(|p| p.get_latest_cells_number())
            .map(|p| p.get_id())
    }

    fn find_best_planet_for_combination(&self, res: common_game::components::resource::ComplexResourceType) -> Option<common_game::utils::ID> {
        self.knowledge.planets.iter()
            .filter(|p| p.get_combinations().contains(&res))
            .max_by_key(|p| p.get_latest_cells_number())
            .map(|p| p.get_id())
    }

    fn conclude_turn(&mut self) -> Result<(), String> {
        while let Ok(msg) = self.rx_orchestrator.try_recv() {
            self.handle_orchestrator_message(msg)?;
        }

        match self.mode{
            crate::explorers::allegory::explorer::ExplorerMode::Killed => {
                Ok(())
            }
            _ => {
                match self.tx_orchestrator.send(
            BagContentResponse {
                explorer_id: self.id,
                bag_content: BagContent::from_bag(
                    &self.bag
                )
            }
        ){
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error sending to orchestrator: {}", e))
        }
            }
        }
        
    }

    fn go_to_safe_planet(&mut self) {
        if let Some(safe_planet) = self
            .knowledge
            .planets
            .iter()
            .find(|p| matches!(p.get_planet_type(), Some(common_game::components::planet::PlanetType::C)))
            .map(|p| p.get_id())
        {
            if safe_planet != self.current_planet_id {
                let _ = self.move_to_planet(safe_planet);
            }
        }
    }


}
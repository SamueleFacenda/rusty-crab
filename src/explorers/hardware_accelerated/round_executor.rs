use std::collections::{HashSet, VecDeque};
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;
use crate::explorers::hardware_accelerated::planning::{get_resource_recipe, get_resource_request};
use super::{GlobalPlanner, LocalPlanner, LocalTask};
use super::{GalaxyKnowledge, OrchestratorCommunicator, PlanetsCommunicator, ExplorerState};

pub(super) struct RoundExecutor<'a> {
    planets_communicator: &'a mut PlanetsCommunicator,
    orchestrator_communicator: &'a OrchestratorCommunicator,
    state: &'a mut ExplorerState,
    new_galaxy: GalaxyKnowledge
}

/// Executes a full round for the explorer
/// Stateless, managed by the global and local planners,
/// tries to build as many dolphins and AI partners as possible
impl<'a> RoundExecutor<'a> {
    pub fn new(
        planets_communicator: &'a mut PlanetsCommunicator,
        orchestrator_communicator: &'a OrchestratorCommunicator,
        state: &'a mut ExplorerState
    ) -> RoundExecutor<'a> {
        RoundExecutor { planets_communicator, orchestrator_communicator, state, new_galaxy: GalaxyKnowledge::new() }
    }

    /// Returns the updated GalaxyKnowledge and the ID of the planet where the explorer ended the round
    pub fn execute_round(mut self) -> Result<(), String> {
        self.explore_galaxy()?; // The only way to get complete galaxy information
        self.update_probabilities(); // Compare with previous state

        self.pursue_explorer_goal()?; // The main behavior is here

        log::info!("Explorer bag: {:?}", self.state.bag);

        self.goto_safest_place()?;
        self.state.knowledge = Some(self.new_galaxy); // Update state at the end of the round
        Ok(())
    }

    fn explore_galaxy(&mut self) -> Result<(), String> {
        let mut explored = HashSet::new();
        while let Some(next_planet) = self.get_best_nearest_unexplored_planet(&explored) {
            self.goto_planet(next_planet)?;

            explored.insert(next_planet);

            self.new_galaxy.add_planet(next_planet);
            self.inspect_current_planet()?;

            // Discover neighbors
            let neighbors = self.orchestrator_communicator.discover_neighbors(self.state.current_planet)?;
            for &neighbor in &neighbors {
                self.new_galaxy.add_planets_connection(self.state.current_planet, neighbor);
            }
        }
        Ok(())
    }

    /// Prefer planets with many connection with explored ones (to explore more on clusters)
    /// Simple BFS
    fn get_best_nearest_unexplored_planet(&self, explored: &HashSet<ID>) -> Option<ID> {
        self.find_nearest_planet(
            |pid| !explored.contains(&pid),
            |pid| {
                self.new_galaxy
                    .get_planet_neighbours(*pid) // Count connections with explored planets
                    .map(|neighbors| neighbors.iter().filter(|n| explored.contains(n)).count())
                    .unwrap_or(0) // If no neighbors, 0 connections
            })
    }

    fn inspect_current_planet(&mut self) -> Result<(), String> {
        let basic = self.planets_communicator.basic_resource_discovery(self.state.current_planet)?;
        let complex = self.planets_communicator.combination_rules_discovery(self.state.current_planet)?;
        let cells = self.planets_communicator.get_available_energy_cells_num(self.state.current_planet)?;
        self.new_galaxy.set_planet_basic_resources(self.state.current_planet, basic);
        self.new_galaxy.set_planet_combination_rules(self.state.current_planet, complex);
        self.new_galaxy.set_n_charged_cells(self.state.current_planet, cells);
        Ok(())
    }

    /// Update probabilities by comparison with previous state
    fn update_probabilities(&mut self) {
        if let Some(knowledge) = self.state.knowledge.as_ref() {
            let n_planets = knowledge.get_n_planets() as u32;
            let sunrays_received = self.new_galaxy.estimate_sunrays_received(knowledge);
            let asteroids_received = self.new_galaxy.estimate_asteroids_received(knowledge);

            self.state.sunray_probability_estimator.update(n_planets, sunrays_received);
            self.state.asteroid_probability_estimator.update(n_planets, asteroids_received);
        }
    }

    fn goto_safest_place(&mut self) -> Result<(), String> {
        let safest = self
            .new_galaxy
            .get_planet_ids()
            .iter()
            .map(|pid| (pid, self.new_galaxy.get_planet_reliability(*pid)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()) // Unwrap safe since reliability is a f32 constant
            .map(|(pid, _)| *pid)
            .ok_or("No planets found in galaxy")?;

        self.goto_planet(safest)
    }

    fn pursue_explorer_goal(&mut self) -> Result<(), String> {
        let global_plan = GlobalPlanner::plan_next_task(self.state);
        let local_plan = LocalPlanner::get_execution_plan(global_plan, &self.state.bag);
        for task in local_plan {
            println!("Executing task: {:?}", task);
            let executed = self.execute_task(task)?;
            if !executed {
                break; // Cannot execute further tasks, it's ok (maybe it's too dangerous)
            }
        }
        Ok(())
    }

    /// Returns Ok(true) if it was possible to execute the task
    fn execute_task(&mut self, task: LocalTask) -> Result<bool, String> {
        match task {
            LocalTask::Produce(resource) => self.produce_resource(resource),
            LocalTask::Generate(resource) => self.generate_resource(resource)
        }
    }

    fn generate_resource(&mut self, resource: BasicResourceType) -> Result<bool, String> {
        let dest = self.find_nearest_planet(
            |planet_id| self.new_galaxy.produces_basic_resource(planet_id, resource)&&
                self.new_galaxy.get_n_charged_cells(planet_id) > 0,
            |planet_id| self.new_galaxy.get_n_charged_cells(*planet_id));// Prefer planets with more energy cells

        if dest.is_none() {
            return Ok(false); // No planet can produce the resource
        }

        self.goto_planet(dest.unwrap())?;

        let mut generated = self.planets_communicator.generate_basic_resource(self.state.current_planet, resource)?;
        self.inspect_current_planet()?; // Update planet state
        if let Some(resource) = generated.take() {
            self.state.bag.insert_basic(resource);
            Ok(true)
        } else {
            // Something changed in the galaxy, try again after inspection (made above)
            self.generate_resource(resource)
        }
    }

    fn produce_resource(&mut self, resource: ComplexResourceType) -> Result<bool, String> {
        let dest = self.find_nearest_planet(
            |planet_id| self.new_galaxy.supports_combination_rule(planet_id, resource) &&
                          self.new_galaxy.get_n_charged_cells(planet_id) > 0,
            |planet_id| self.new_galaxy.get_n_charged_cells(*planet_id));// Prefer planets with more energy cells

        if dest.is_none() {
            return Ok(false); // No planet can produce the resource
        }

        self.goto_planet(dest.unwrap())?;

        let ingredients = self.state.bag.get_recipe_ingredients(resource);
        if ingredients.is_none() {
            return Ok(false); // Not enough resources to combine
        }
        let (a, b) = ingredients.unwrap();
        let msg = get_resource_request(resource, a, b);
        let generated = self.planets_communicator.combine_resources(self.state.current_planet, msg)?;

        self.inspect_current_planet()?; // Update planet state
        match generated {
            Ok(resource) => {
                self.state.bag.insert_complex(resource);
                Ok(true)
            },
            Err((_comm_err, res_a, res_b)) => {
                 // Combination failed, re-insert resources back into bag
                 self.state.bag.res.entry(res_a.get_type()).or_default().push(res_a);
                 self.state.bag.res.entry(res_b.get_type()).or_default().push(res_b);
                 // Something changed in the galaxy, try again after inspection (made above)
                 self.produce_resource(resource)
            }
        }
    }

    /// BFS to find nearest planet satisfying predicate, then select the best one according to discriminant (max by)
    fn find_nearest_planet<B: Ord>(&self, predicate: impl Fn(ID) -> bool, discriminant: impl Fn(&ID) -> B) -> Option<ID> {
        let mut candidates: Vec<ID> = Vec::new();
        let mut best_distance = i32::MAX;

        let mut queue: VecDeque<(ID, i32)> = VecDeque::new();
        queue.push_back((self.state.current_planet, 0));
        let mut visited: HashSet<ID> = HashSet::new();
        while !queue.is_empty() {
            let (planet_id, distance) = queue.pop_front().unwrap();
            if distance > best_distance {
                continue; // Drain the queue with all the same distance planets
            }
            if visited.contains(&planet_id) {
                continue;
            }
            visited.insert(planet_id);

            if predicate(planet_id) {
                candidates.push(planet_id);
                best_distance = distance; // Update best distance
                continue;
            }

            if let Some(neighbors) = self.new_galaxy.get_planet_neighbours(planet_id) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        queue.push_back((neighbor, distance + 1));
                    }
                }
            }
        }

        candidates
            .into_iter()
            .max_by_key(discriminant)
    }

    /// Evaluate the risk of using energy on a planet
    fn evaluate_energy_usage_risk(&self, planet_id: ID) -> f32 {
        // Placeholder for actual risk evaluation logic
        0.0
    }

    fn goto_planet(&mut self, planet_id: ID) -> Result<(), String> {
        let path = self.get_path_to_planet(planet_id)?;
        for p in path {
            let opt_sender = self.orchestrator_communicator.travel_to_planet(self.state.current_planet, p)?;
            if opt_sender.is_none() {
                return Err(format!("Travel to planet {} failed", p));
            }
            self.planets_communicator.add_planet(p, opt_sender.unwrap());
            self.planets_communicator.set_current_planet(p);
            self.state.current_planet = p;
        }
        Ok(())
    }

    /// BFS to find path to planet (path doesn't include starting planet)
    fn get_path_to_planet(&self, planet_id: ID) -> Result<Vec<ID>, String> {
        let visited: HashSet<ID> = HashSet::new();
        let mut queue: VecDeque<Vec<ID>> = VecDeque::new();
        queue.push_back(vec![self.state.current_planet]);
        while !queue.is_empty() {
            let path = queue.pop_front().unwrap();
            let node = *path.last().unwrap();
            if node == planet_id {
                return Ok(path[1..].to_vec()); // Exclude starting planet
            }
            if !visited.contains(&node) {
                if let Some(neighbours) = self.new_galaxy.get_planet_neighbours(node) {
                    for &neighbor in neighbours {
                        let mut new_path = path.clone();
                        new_path.push(neighbor);
                        queue.push_back(new_path);
                    }
                }
            }
        }
        Err(format!("No path found to planet {}", planet_id))
    }
}

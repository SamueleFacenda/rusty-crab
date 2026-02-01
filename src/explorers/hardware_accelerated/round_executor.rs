use std::collections::{HashSet, VecDeque};
use common_game::utils::ID;
use crate::explorers::hardware_accelerated::communication::PlanetLoggingSender;
use crate::explorers::hardware_accelerated::explorer::ExplorerState;
use super::{OrchestratorCommunicator, PlanetsCommunicator, GalaxyKnowledge};

pub(super) struct RoundExecutor<'a> {
    planets_communicator: &'a mut PlanetsCommunicator,
    orchestrator_communicator: &'a OrchestratorCommunicator,
    state: &'a mut ExplorerState,
    new_galaxy: GalaxyKnowledge
}

impl<'a> RoundExecutor<'a> {
    pub fn new(planets_communicator: &'a mut PlanetsCommunicator, orchestrator_communicator: &'a OrchestratorCommunicator , state: &'a mut ExplorerState) -> RoundExecutor<'a> {
        RoundExecutor {
            planets_communicator,
            orchestrator_communicator,
            state,
            new_galaxy: GalaxyKnowledge::new(),
        }
    }

    /// Returns the updated GalaxyKnowledge and the ID of the planet where the explorer ended the round
    pub fn execute_round(mut self) -> Result<(), String> {
        self.explore_galaxy()?; // The only way to get complete galaxy information
        self.update_probabilities(); // Compare with previous state

        self.do_fun_activities()?; // The main behavior is here

        self.goto_safest_place()?;
        self.state.knowledge = Some(self.new_galaxy); // Update state at the end of the round
        Ok(())
    }

    fn explore_galaxy(&mut self) -> Result<(), String> {
        let mut explored = HashSet::new();
        while let Some(next_planet) = self.get_best_nearest_unexplored_planet(&explored) {
            self.goto_planet(next_planet)?;

            explored.insert(next_planet);

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
        let mut unexplored: Vec<ID> = Vec::new();
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

            if !explored.contains(&planet_id) {
                unexplored.push(planet_id);
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

        unexplored.into_iter().map(|pid| (pid,
            self.new_galaxy.get_planet_neighbours(pid) // Count connections with explored planets
                .map(|neighbors| neighbors.iter()
                    .filter(|n| explored.contains(n))
                    .count())
                .unwrap_or(0))) // If no neighbors, 0 connections
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(pid, _)| pid)
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
        let safest = self.new_galaxy.get_planet_ids().iter()
            .map(|pid| (pid, self.new_galaxy.get_planet_reliability(pid)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(pid, _)| *pid)
            .ok_or("No planets found in galaxy")?;

        self.goto_planet(safest)
    }

    fn do_fun_activities(&mut self) -> Result<(), String> {
        // Placeholder for fun activities
        Ok(())
    }

    fn goto_planet(&mut self, planet_id: ID) -> Result<(), String> {
        let path = self.get_path_to_planet(planet_id)?;
        for p in path {
            let opt_sender = self.orchestrator_communicator.travel_to_planet(self.state.current_planet, p)?;
            if opt_sender.is_none() {
                return Err(format!("Travel to planet {} failed", p));
            }
            self.planets_communicator.add_planet(p, PlanetLoggingSender::new(opt_sender.unwrap(), p));
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

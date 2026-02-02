use common_game::components::planet::PlanetType;
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::utils::ID;
use std::collections::{HashMap, HashSet, VecDeque};

/// Stores everything that the explorer knows about a galaxy:
/// - current_state: The current state of its task (Exploring, Collecting, Finished, Failed)
/// - planets: All the knowledge about the planets (see [`PlanetKnowledge`])
/// - pending_resources: list of what planet is producing what
pub struct ExplorerKnowledge {
    current_state: StrategyState,
    current_target_planet: Option<ID>,
    pub(crate) planets: Vec<PlanetKnowledge>,
}

#[derive(Debug, PartialEq)]
pub enum StrategyState {
    Exploring, // Obsolete with new orchestrator structure
    Collecting,
    Crafting,
    Finished,
    Failed,
}

/// Stores everything that is known about a planet.
/// This includes:
/// - ID
/// - Planet type (A-D)
/// - is_destroyed: ...
/// - Unordered list of neighbors
/// - List of resources and combinations
pub(crate) struct PlanetKnowledge {
    id: ID,
    planet_type: PlanetType,
    is_destroyed: bool,
    neighbors: HashSet<ID>,
    resource_type: HashSet<BasicResourceType>,
    combinations: HashSet<ComplexResourceType>,
    latest_cells_number: u32,
}
impl PlanetKnowledge {
    pub(crate) fn new(
        id: ID,
        planet_type: PlanetType,
        neighbors: HashSet<ID>,
        resource_type: HashSet<BasicResourceType>,
        combinations: HashSet<ComplexResourceType>,
        latest_cells_number: u32,
    ) -> Self {
        PlanetKnowledge {
            id,
            planet_type,
            is_destroyed: false,
            neighbors,
            resource_type,
            combinations,
            latest_cells_number,
        }
    }

    // getters
    pub(crate) fn get_id(&self) -> ID {
        self.id
    }

    pub(crate) fn get_planet_type(&self) -> PlanetType {
        self.planet_type
    }

    pub(crate) fn get_is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    pub(crate) fn get_neighbors(&self) -> &HashSet<ID> {
        &self.neighbors
    }

    pub(crate) fn get_resource_type(&self) -> &HashSet<BasicResourceType> {
        &self.resource_type
    }

    pub(crate) fn get_combinations(&self) -> &HashSet<ComplexResourceType> {
        &self.combinations
    }

    pub(crate) fn get_latest_cells_number(&self) -> u32 {
        self.latest_cells_number
    }


}

impl Default for ExplorerKnowledge {
    fn default() -> Self {
        ExplorerKnowledge {
            current_state: StrategyState::Exploring,
            current_target_planet: None,
            planets: vec![],
        }
    }
}
impl ExplorerKnowledge {
    fn new(
        current_state: StrategyState,
        planets: Vec<PlanetKnowledge>,
        pending_resources: HashSet<(ID, ResourceType)>,
    ) -> Self {
        ExplorerKnowledge {
            current_state,
            current_target_planet: None,
            planets,
        }
    }

    pub(crate) fn get_resource_by_id(&self, id: ID) -> Option<HashSet<BasicResourceType>> {
        match self.planets.iter().find(|planet| planet.id == id) {
            None => None,
            Some(planet_knowledge) => Some(planet_knowledge.resource_type.clone()),
        }
    }

    pub(crate) fn get_combinations_by_id(&self, id: ID) -> Option<HashSet<ComplexResourceType>> {
        match self.planets.iter().find(|planet| planet.id == id) {
            None => None,
            Some(planet_knowledge) => Some(planet_knowledge.combinations.clone()),
        }
    }

    pub(crate) fn update_neighbors(&mut self, planet_id: ID, neighbors: HashSet<ID>) {
        // Modify or add given vector
        if let Some(planet) = self.planets.iter_mut().find(|p| p.id == planet_id) {
            planet.neighbors = neighbors;
        }
    }

    pub(crate) fn update_planet_resource(
        &mut self,
        planet_id: ID,
        resources: HashSet<BasicResourceType>,
    ) {
        // Same
        if let Some(planet) = self.planets.iter_mut().find(|p| p.id == planet_id) {
            planet.resource_type = resources;
        }
    }

    pub(crate) fn add_cell(&mut self, planet_id: ID, value: u32) {
        if let Some(planet) = self.planets.iter_mut().find(|p| p.id == planet_id) {
            planet.latest_cells_number = value;
        }
    }

    pub(crate) fn update_killed_planet(&mut self, planet_id: ID) {
        if let Some(planet) = self.planets.iter_mut().find(|p| p.id == planet_id) {
            planet.is_destroyed = true;
        }
    }

    pub(crate) fn update_planet_combinations(
        &mut self,
        planet_id: ID,
        combination_list: HashSet<ComplexResourceType>,
    ) {
        {
            if let Some(planet) = self.planets.iter_mut().find(|p| p.id == planet_id) {
                planet.combinations = combination_list;
            }
        }
    }

    pub(crate) fn get_explored_planets(&self) -> HashSet<ID> {
        let mut visited_planets: HashSet<ID> = HashSet::new();
        for planet in &self.planets {
            visited_planets.insert(planet.id);
        }
        visited_planets
    }

    /// Returns unexplored planets from knowledge
    pub(crate) fn get_unexplored_planets(&self) -> HashSet<ID> {
        let mut unexplored_planets: HashSet<ID> = HashSet::new();
        let mut visited_planets: HashSet<ID> = HashSet::new();
        for planet in &self.planets {
            unexplored_planets.extend(&planet.neighbors);
            visited_planets.insert(planet.id);
        }
        unexplored_planets
            .difference(&visited_planets)
            .cloned()
            .collect()
    }

    /// Returns unexplored planets for each turn from hashset
    pub(crate) fn get_unexplored_from_hash(&self, explored_planets: &HashSet<ID>) -> HashSet<ID> {
        let mut unexplored_planets: HashSet<ID> = HashSet::new();
        for planet_id in explored_planets {
            if let Some(pk) = self.get_planet_knowledge(*planet_id) {
                unexplored_planets.extend(&pk.neighbors);
            }
        }
        unexplored_planets
            .difference(explored_planets)
            .cloned()
            .collect()
    }

    pub(crate) fn get_planet_knowledge(&self, planet_id: ID) -> Option<&PlanetKnowledge> {
        self.planets.iter().find(|planet| planet.id == planet_id)
    }

    pub(crate) fn update_state(&mut self, state: StrategyState) {
        self.current_state = state;
    }

    pub(crate) fn get_current_state(&self) -> &StrategyState { &self.current_state}

    pub(crate) fn get_target_planet(&self) -> Option<ID> {
        self.current_target_planet
    }

    pub(crate) fn set_destination(&mut self, planet: Option<ID>) {
        self.current_target_planet = planet;
    }

    pub(crate) fn get_neighbors(&self, planet: ID) -> Option<&HashSet<ID>> {
        self.get_planet_knowledge(planet)
            .map(|planet_knowledge| planet_knowledge.get_neighbors())
    }

    /// Verifies if a certain planet can generate or produce a certain resource.
    pub(crate) fn can_planet_produce(&self, planet: ID, resource: ResourceType) -> bool {
        if let Some(planet_knowledge) = self.get_planet_knowledge(planet) {
            match resource {
                ResourceType::Basic(basic_resource) => {
                    if planet_knowledge
                        .get_resource_type()
                        .contains(&basic_resource)
                    {
                        return true;
                    }
                }
                ResourceType::Complex(complex_resource) => {
                    if planet_knowledge
                        .get_combinations()
                        .contains(&complex_resource)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Determines the fastest path to reach a certain planet and sets the course
    pub(crate) fn get_next_hop(&self, starting_planet: ID) -> ID {
        // If there's no target, stay at current planet
        let target = match self.current_target_planet {
            Some(id) => id,
            None => return starting_planet,
        };

        // Ifalready at target, return
        if starting_planet == target {
            return starting_planet;
        }

        // BFS again
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parents = HashMap::new();

        queue.push_back(starting_planet);
        visited.insert(starting_planet);

        while let Some(curr) = queue.pop_front() {
            if let Some(pk) = self.get_planet_knowledge(curr) {
                for &neighbor in pk.get_neighbors() {
                    if neighbor == target {
                        // Found target
                        if curr == starting_planet {
                            return neighbor;
                        }

                        let mut trace = curr;
                        while let Some(&parent) = parents.get(&trace) {
                            if parent == starting_planet {
                                return trace;
                            }
                            trace = parent;
                        }
                        return trace;
                    }

                    if !visited.contains(&neighbor) && self.get_planet_knowledge(neighbor).is_some()
                    {
                        visited.insert(neighbor);
                        parents.insert(neighbor, curr);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // No path found, return starting planet
        starting_planet
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_planet_knowledge_creation() {
        let id: ID = 3;
        let planet_type = PlanetType::C;
        let neighbors = HashSet::new();
        let resource_type = HashSet::new();
        let combinations = HashSet::new();
        let latest_cells_number = 0;

        let planet_knowledge = PlanetKnowledge::new(
            id,
            planet_type,
            neighbors.clone(),
            resource_type.clone(),
            combinations.clone(),
            latest_cells_number,
        );

        assert_eq!(planet_knowledge.id, id);
        assert!(!planet_knowledge.is_destroyed);
        assert_eq!(planet_knowledge.neighbors, neighbors);
        assert_eq!(planet_knowledge.resource_type, resource_type);
        assert_eq!(planet_knowledge.combinations, combinations);
        assert_eq!(planet_knowledge.latest_cells_number, latest_cells_number);
    }

    #[test]
    fn test_explorer_knowledge_default() {
        let explorer_knowledge = ExplorerKnowledge::default();

        assert_eq!(explorer_knowledge.current_state, StrategyState::Exploring);
        assert!(explorer_knowledge.planets.is_empty());
    }

    #[test]
    fn test_explorer_knowledge_add_planet() {
        let mut explorer_knowledge = ExplorerKnowledge::default();
        let id: ID = 4;
        let planet_type = PlanetType::A;
        let neighbors = HashSet::new();
        let resource_type = HashSet::new();
        let combinations = HashSet::new();
        let latest_cells_number = 0;

        let planet_knowledge = PlanetKnowledge::new(
            id,
            planet_type,
            neighbors,
            resource_type,
            combinations,
            latest_cells_number,
        );
        explorer_knowledge.planets.push(planet_knowledge);

        assert_eq!(explorer_knowledge.planets.len(), 1);
        assert_eq!(explorer_knowledge.planets[0].id, id);
    }

    #[test]
    fn test_get_resource_by_id() {
        let mut explorer_knowledge = ExplorerKnowledge::default();
        let id: ID = 5;
        let planet_type = PlanetType::B;
        let neighbors = HashSet::new();
        let resource_type = HashSet::from([BasicResourceType::Carbon]);
        let combinations = HashSet::new();
        let latest_cells_number = 0;

        let planet_knowledge = PlanetKnowledge::new(
            id,
            planet_type,
            neighbors,
            resource_type.clone(),
            combinations,
            latest_cells_number,
        );
        explorer_knowledge.planets.push(planet_knowledge);

        let resources = explorer_knowledge.get_resource_by_id(id).unwrap();
        assert_eq!(resources, resource_type);
    }

    #[test]
    fn test_update_neighbors() {
        let mut explorer_knowledge = ExplorerKnowledge::default();
        let id: ID = 6;
        let planet_type = PlanetType::D; // Replace with an actual variant
        let neighbors = HashSet::new();
        let resource_type = HashSet::new();
        let combinations = HashSet::new();
        let latest_cells_number = 0;

        let planet_knowledge = PlanetKnowledge::new(
            id,
            planet_type,
            neighbors.clone(),
            resource_type,
            combinations,
            latest_cells_number,
        );
        explorer_knowledge.planets.push(planet_knowledge);

        let new_neighbors = HashSet::from([3]);
        explorer_knowledge.update_neighbors(id, new_neighbors.clone());

        assert_eq!(explorer_knowledge.planets[0].neighbors, new_neighbors);
    }

    #[test]
    fn test_update_killed_planet() {
        let mut explorer_knowledge = ExplorerKnowledge::default();
        let id: ID = 7;
        let planet_type = PlanetType::A;
        let neighbors = HashSet::new();
        let resource_type = HashSet::new();
        let combinations = HashSet::new();
        let latest_cells_number = 0;

        let planet_knowledge = PlanetKnowledge::new(
            id,
            planet_type,
            neighbors,
            resource_type,
            combinations,
            latest_cells_number,
        );
        explorer_knowledge.planets.push(planet_knowledge);

        explorer_knowledge.update_killed_planet(id);
        assert!(explorer_knowledge.planets[0].is_destroyed);
    }

    #[test]
    fn test_get_unexplored_from_hash() {
        let mut explorer_knowledge = ExplorerKnowledge::default();
        // Line topology
        // Planet 1 neighbors with 2 and 3
        let pk1 = PlanetKnowledge::new(
            1,
            PlanetType::A,
            HashSet::from([2, 3]),
            HashSet::new(),
            HashSet::new(),
            0,
        );
        
        // Planet 2 neighbors with 1
        let pk2 = PlanetKnowledge::new(
            2,
            PlanetType::B,
            HashSet::from([1]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

         // Planet 3 neighbors with 1
         let pk3 = PlanetKnowledge::new(
            3,
            PlanetType::C,
            HashSet::from([1]),
            HashSet::new(),
            HashSet::new(),
            0,
        );

        explorer_knowledge.planets.push(pk1);
        explorer_knowledge.planets.push(pk2);
        explorer_knowledge.planets.push(pk3);

        // Case 1: Only planet 1 is explored. 
        let explored = HashSet::from([1]);
        let unexplored = explorer_knowledge.get_unexplored_from_hash(&explored);
        let expected = HashSet::from([2, 3]);
        assert_eq!(unexplored, expected);

        // Case 2: Planet 1 and 2 are explored.
        let explored = HashSet::from([1, 2]);
        let unexplored = explorer_knowledge.get_unexplored_from_hash(&explored);
        let expected = HashSet::from([3]);
        assert_eq!(unexplored, expected);
        
         // Case 3: All explored
        let explored = HashSet::from([1, 2, 3]);
        let unexplored = explorer_knowledge.get_unexplored_from_hash(&explored);
        assert!(unexplored.is_empty());
    }
}

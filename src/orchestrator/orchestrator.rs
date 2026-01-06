use std::collections::{HashMap, HashSet};
use common_game::components::planet::{Planet};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};
use crate::orchestrator::example_explorer::Explorer;

struct Orchestrator{
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    time: u32,

    // Auto/manual
    mode: OrchestratorMode,

    // Asteroids and sunrays are produced randomly for each planet following a given pattern.
    // For example, [(0,0.1), (10,0.2), (20, 0.3)] means that the probability of an event is 10% until time = 9, 20% until time = 19, 30% from 20 on.
    // My idea was to have a slow start for asteroids which increase to 1 (end of the solar system-like), while keeping sunrays constant.
    // Let me know what you think of this implementation; can be changed, but it's as simple as I could get it to be.
    asteroid_pattern: Vec<(u32, f32)>,
    sunray_pattern: Vec<(u32, f32)>,

    // List of planets in the galaxy and topology
    planets: HashMap<ID, PlanetHandle>,

    // List of explorers
    explorers: HashMap<ID, ExplorerHandle>,
}
enum OrchestratorMode{
    Auto,
    Manual,
}

// struct used to handle the list of planets.
// This is partly redundant as ID is stored twice,
// But the alternative is to store topology in a separate
// struct which would also require ID as key
// Can be changed if you find a better way
pub struct PlanetHandle {
    planet: Planet,
    neighbors: HashSet<ID>,
}

// Struct to hold explorers;
// Again ID is probably also in the explorer struct,
// As well as the state. There is no clear interface, but we should agree on the main content of the struct.
// todo! Agree on the explorer fields
pub struct ExplorerHandle {
    explorer: Explorer, // Example implementation defined in example_explorer.rs
    current_planet: ID,
    tx: Sender<OrchestratorToExplorer>,
    rx: Receiver<ExplorerToOrchestrator<()>>,  // To determine what this parameter should be
    state: ExplorerState,
}

pub enum ExplorerState {
    Autonomous,
    Manual,
    Stopped,
    Destroyed,
}


impl Orchestrator{
    pub fn new(
               mode: OrchestratorMode,
               asteroid_pattern: Vec<(u32, f32)>,
               sunray_pattern: Vec<(u32, f32)>,
               planets: HashMap<ID, PlanetHandle>,
               explorers: HashMap<ID, ExplorerHandle>) -> Self{
        Orchestrator{
            time: 0,
            mode,
            asteroid_pattern,
            sunray_pattern,
            planets,
            explorers
        }
    }

    pub fn initialize(){
        todo!()
        // initialization functions
        // Add planets we bought -> create connections
        // logger?
        // Start planet AI and explorer AI
        // execute first loop
    }

    pub fn add_planet(&mut self, planet: Planet) -> Result<(), String> {
        let id = planet.id();
        if self.planets.contains_key(&id) {
            return Err(format!("Planet {} already exists", id));
        }

        self.planets.insert(id, PlanetHandle {
            planet,
            neighbors: HashSet::new(),
        });
        Ok(())
    }

    // Function to connect planets.
    // All planets should already be in the list from initialization
    fn connect_planets (&mut self, planet1_id: ID, planet2_id: ID) -> Result<(), String> {
        if planet1_id == planet2_id {
            return Ok(())
        }
        
        if !self.planets.contains_key(&planet1_id) {
            return Err(format!("Planet {} does not exist", planet1_id));
        }
        if !self.planets.contains_key(&planet2_id) {
            return Err(format!("Planet {} does not exist", planet2_id));
        }

        // Unwrap is safe after check
        self.planets.get_mut(&planet1_id).unwrap().neighbors.insert(planet2_id);
        self.planets.get_mut(&planet2_id).unwrap().neighbors.insert(planet1_id);
        
        Ok(())
    }

    fn remove_planet_connections(&mut self, planet_id: ID)-> Result<(), String>{
        if !self.planets.contains_key(&planet_id) {
            return Err(format!("Planet {} does not exist", planet_id));
        }

        for (id, planet) in self.planets.iter_mut() {
            if *id == planet_id {
                planet.neighbors.clear();
            } else {
                planet.neighbors.remove(&planet_id);
            }
        }
        Ok(())
    }
}


impl Default for Orchestrator{
    fn default() -> Self {
        Orchestrator{
            time: 0,
            mode: OrchestratorMode::Auto,
            asteroid_pattern: vec![(0, 0.0)],
            sunray_pattern: vec![(0, 0.0)],
            planets: Default::default(),
            explorers: Default::default(),
        }
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    use common_game::components::planet::{PlanetAI, PlanetState, PlanetType, DummyPlanetState};
    use common_game::components::resource::{BasicResourceType, Combinator, Generator};
    use common_game::components::sunray::Sunray;
    use common_game::components::rocket::Rocket;
    use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
    use common_game::protocols::orchestrator_planet::OrchestratorToPlanet;
    use crossbeam_channel::unbounded;

    struct DummyAI;
    impl PlanetAI for DummyAI {
        fn handle_sunray(&mut self, _state: &mut PlanetState, _gen: &Generator, _comb: &Combinator, _sunray: Sunray) {}
        fn handle_asteroid(&mut self, _state: &mut PlanetState, _gen: &Generator, _comb: &Combinator) -> Option<Rocket> { None }
        fn handle_internal_state_req(&mut self, state: &mut PlanetState, _gen: &Generator, _comb: &Combinator) -> DummyPlanetState { state.to_dummy() }
        fn handle_explorer_msg(&mut self, _state: &mut PlanetState, _gen: &Generator, _comb: &Combinator, _msg: ExplorerToPlanet) -> Option<PlanetToExplorer> { None }
    }

    fn create_dummy_planet(id: ID) -> Planet {
        let (tx_orch, rx_orch) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (_tx_expl, rx_expl) = unbounded();
        
        Planet::new(
            id,
            PlanetType::A,
            Box::new(DummyAI),
            vec![BasicResourceType::Oxygen],
            vec![],
            (rx_orch, tx_planet),
            rx_expl
        ).unwrap()
    }

    #[test]
    fn test_add_planet() {
        let mut orch = Orchestrator::default();
        let p1 = create_dummy_planet(1);
        assert!(orch.add_planet(p1).is_ok());
        assert!(orch.planets.contains_key(&1));
        
        let p1_dup = create_dummy_planet(1);
        assert!(orch.add_planet(p1_dup).is_err());
    }

    #[test]
    fn test_connect_planets() {
        let mut orch = Orchestrator::default();
        orch.add_planet(create_dummy_planet(1)).unwrap();
        orch.add_planet(create_dummy_planet(2)).unwrap();
        
        assert!(orch.connect_planets(1, 2).is_ok());
        
        assert!(orch.planets.get(&1).unwrap().neighbors.contains(&2));
        assert!(orch.planets.get(&2).unwrap().neighbors.contains(&1));
        
        assert!(orch.connect_planets(1, 99).is_err());
    }

    #[test]
    fn test_remove_planet_connections() {
        let mut orch = Orchestrator::default();
        orch.add_planet(create_dummy_planet(1)).unwrap();
        orch.add_planet(create_dummy_planet(2)).unwrap();
        orch.add_planet(create_dummy_planet(3)).unwrap();
        
        orch.connect_planets(1, 2).unwrap();
        orch.connect_planets(1, 3).unwrap();
        
        assert!(orch.remove_planet_connections(1).is_ok());
        
        assert!(!orch.planets.get(&2).unwrap().neighbors.contains(&1));
        assert!(!orch.planets.get(&3).unwrap().neighbors.contains(&1));
        assert!(orch.planets.get(&1).unwrap().neighbors.is_empty());
    }
}
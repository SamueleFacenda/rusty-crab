use std::collections::{HashMap, HashSet};
use common_game::components::planet::{Planet};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender, unbounded};
use crate::orchestrator::example_explorer::{ExampleExplorer, Explorer};
use air_fryer;


struct Orchestrator <T: Explorer>{
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    time: u32,

    // Auto/manual
    mode: OrchestratorMode,

    // List of planets in the galaxy and topology
    planets: HashMap<ID, PlanetHandle>,

    // List of explorers
    explorers: HashMap<ID, ExplorerHandle<T>>,
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
    pub tx: Sender<OrchestratorToPlanet>,
    pub rx: Receiver<PlanetToOrchestrator>,
    pub tx_explorer: Sender<ExplorerToPlanet>,
}

// Struct to hold explorers;
// Again ID is probably also in the explorer struct,
// As well as the state. Created explorer trait.
pub struct ExplorerHandle <T: Explorer> {
    explorer: T, // Example implementation defined in example_explorer.rs
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


impl<T: Explorer> Orchestrator<T>{
    pub fn new(
               mode: OrchestratorMode,
               planets: HashMap<ID, PlanetHandle>,
               explorers: HashMap<ID, ExplorerHandle<T>>) -> Self{
        Orchestrator{
            time: 0,
            mode,
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

    fn get_asteroid_p(&self) -> f32 {
        // Sigmoid starting from 0.01
        let p_start = 0.01f32;
        let t0 = (1.0 / p_start) * ((1.0 - p_start) / p_start).ln();
        1.0 / (1.0 + (-p_start * (self.time as f32 - t0)).exp())
    }

    fn get_sunray_p(&self) -> f32 {
        // Constant
        0.1
    }
    pub fn add_planet(
        &mut self,
        planet: Planet,
        tx: Sender<OrchestratorToPlanet>,
        rx: Receiver<PlanetToOrchestrator>,
        tx_explorer: Sender<ExplorerToPlanet>,
    ) -> Result<(), String> {
        let id = planet.id();
        if self.planets.contains_key(&id) {
            return Err(format!("Planet {} already exists", id));
        }

        self.planets.insert(id, PlanetHandle {
            planet,
            neighbors: HashSet::new(),
            tx,
            rx,
            tx_explorer,
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

    // Still uses 2.0.0, will stop working if updated
    fn create_planet_1(
        &mut self,
        id: ID
    ) -> Result<(), String> {
        let (tx_orchestrator, rx_orchestrator) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_explorer, rx_explorer) = unbounded();

        let p = the_compiler_strikes_back::planet::create_planet(
            rx_orchestrator,
            tx_planet,
            rx_explorer,
            id
        );
        self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    }

    // Functionally ok, but the package is outdated therefore the types are not matching
    // arguments to this method are incorrect [E0308]
    // Note: two different versions of crate `common_game` are being used; two types coming from
    // two different versions of the same crate are different types even if they look the same
    
    // fn create_planet_2(
    //     &mut self,
    //     id: ID
    // ) -> Result<(), String> {
    //     let (tx_orchestrator, rx_orchestrator) = unbounded();
    //     let (tx_planet, rx_planet) = unbounded();
    //     let (tx_explorer, rx_explorer) = unbounded();
    //     let p = match air_fryer::create_planet(
    //         id,
    //         air_fryer::PlanetAI::new(),
    //         (rx_orchestrator, tx_planet), // To be checked
    //         rx_explorer,
    //
    //     ){
    //         Ok(p) => p,
    //         Err(e) => return Err(e)
    //     };
    //     self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    // }

    // Same thing wrong version

    // fn create_planet_3(
    //     &mut self,
    //     id: ID
    // ) -> Result<(), String> {
    //     let (tx_orchestrator, rx_orchestrator) = unbounded();
    //     let (tx_planet, rx_planet) = unbounded();
    //     let (tx_explorer, rx_explorer) = unbounded();
    //
    //     let p = rustrelli::create_planet(
    //         id,
    //         rx_orchestrator,
    //         tx_planet,
    //         rx_explorer,
    //         rustrelli::ExplorerRequestLimit::None,
    //     );
    //
    //     self.add_planet(p, tx_orchestrator, rx_planet, tx_explorer)
    // }
}


impl<T: Explorer> Default for Orchestrator<T>{
    fn default() -> Self {
        Orchestrator{
            time: 0,
            mode: OrchestratorMode::Auto,
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

    fn create_dummy_planet(id: ID) -> (Planet, Sender<OrchestratorToPlanet>, Receiver<PlanetToOrchestrator>, Sender<ExplorerToPlanet>) {
        let (tx_orch, rx_orch) = unbounded();
        let (tx_planet, rx_planet) = unbounded();
        let (tx_expl, rx_expl) = unbounded();
        
        let p = Planet::new(
            id,
            PlanetType::A,
            Box::new(DummyAI),
            vec![BasicResourceType::Oxygen],
            vec![],
            (rx_orch, tx_planet),
            rx_expl
        ).unwrap();

        (p, tx_orch, rx_planet, tx_expl)
    }

    #[test]
    fn test_add_planet() {
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let (p1, tx, rx, tx_e) = create_dummy_planet(1);
        assert!(orchestrator.add_planet(p1, tx, rx, tx_e).is_ok());
        assert!(orchestrator.planets.contains_key(&1));
        
        let (p1_dup, tx, rx, tx_e) = create_dummy_planet(1);
        assert!(orchestrator.add_planet(p1_dup, tx, rx, tx_e).is_err());
    }

    #[test]
    fn test_connect_planets() {
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let (p1, tx1, rx1, tx_e1) = create_dummy_planet(1);
        orchestrator.add_planet(p1, tx1, rx1, tx_e1).unwrap();
        let (p2, tx2, rx2, tx_e2) = create_dummy_planet(2);
        orchestrator.add_planet(p2, tx2, rx2, tx_e2).unwrap();
        
        assert!(orchestrator.connect_planets(1, 2).is_ok());
        
        assert!(orchestrator.planets.get(&1).unwrap().neighbors.contains(&2));
        assert!(orchestrator.planets.get(&2).unwrap().neighbors.contains(&1));
        
        assert!(orchestrator.connect_planets(1, 99).is_err());
    }

    #[test]
    fn test_remove_planet_connections() {
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let (p1, tx1, rx1, tx_e1) = create_dummy_planet(1);
        orchestrator.add_planet(p1, tx1, rx1, tx_e1).unwrap();
        let (p2, tx2, rx2, tx_e2) = create_dummy_planet(2);
        orchestrator.add_planet(p2, tx2, rx2, tx_e2).unwrap();
        let (p3, tx3, rx3, tx_e3) = create_dummy_planet(3);
        orchestrator.add_planet(p3, tx3, rx3, tx_e3).unwrap();

        orchestrator.connect_planets(1, 2).unwrap();
        orchestrator.connect_planets(1, 3).unwrap();
        
        assert!(orchestrator.remove_planet_connections(1).is_ok());
        
        assert!(!orchestrator.planets.get(&2).unwrap().neighbors.contains(&1));
        assert!(!orchestrator.planets.get(&3).unwrap().neighbors.contains(&1));
        assert!(orchestrator.planets.get(&1).unwrap().neighbors.is_empty());
        assert!(orchestrator.planets.get(&1).unwrap().neighbors.is_empty());
    }

    #[test]
    fn verify_probabilities(){
        // verify the initial value and that the probability tends to 1
        let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
        let asteroid_0 = orchestrator.get_asteroid_p();
        let sunray_0 = orchestrator.get_sunray_p();
        // println!("0: {}, time: {}", asteroid_0, orchestrator.time);
        assert!(asteroid_0 < 0.01001);
        assert!(asteroid_0 > 0.0099);
        assert_eq!(sunray_0, 0.1);
        orchestrator.time = 100;
        let asteroid_100 = orchestrator.get_asteroid_p();
        let sunray_100 = orchestrator.get_sunray_p();
        // println!("100: {}, time: {}", asteroid_100, orchestrator.time);
        assert!(asteroid_100 <= 0.03);
        assert!(asteroid_100 >= 0.02);
        assert_eq!(sunray_100, 0.1);
        orchestrator.time = 1000;
        let asteroid_1000 = orchestrator.get_asteroid_p();
        let sunray_1000 = orchestrator.get_sunray_p();
        // println!("1000: {}, time: {}", asteroid_1000, orchestrator.time);
        assert!(asteroid_1000 >= 0.9);
        assert_eq!(sunray_1000, 0.1);
    }
}
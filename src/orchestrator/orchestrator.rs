use std::collections::{HashMap, HashSet};
use std::thread;

use common_game::components::planet::{Planet, PlanetState};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender, unbounded};
use log::error;
use crate::orchestrator::example_explorer::{Explorer};
use crate::app::AppConfig;

#[allow(dead_code)]
pub(crate) struct Orchestrator <T: Explorer>{
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    time: u32,

    // Auto/manual
    mode: OrchestratorMode,

    // List of planets in the galaxy and topology
    pub(crate) planets: HashMap<ID, PlanetHandle>,

    // List of explorers
    explorers: HashMap<ID, ExplorerHandle<T>>,
}
pub(crate) enum OrchestratorMode{
    Auto,
    Manual,
}

// struct used to handle the list of planets.
// This is partly redundant as ID is stored twice,
// But the alternative is to store topology in a separate
// struct which would also require ID as key
// Can be changed if you find a better way
pub struct PlanetHandle {
    pub(crate) planet: Option<Planet>,
    pub(crate) neighbors: HashSet<ID>,
    pub (crate) thread_handle: Option<thread::JoinHandle<()>>,
    pub tx: Sender<OrchestratorToPlanet>,
    pub rx: Receiver<PlanetToOrchestrator>,
    pub tx_explorer: Sender<ExplorerToPlanet>,
}

// Struct to hold explorers;
// Again ID is probably also in the explorer struct,
// As well as the state. Created explorer trait.
pub struct ExplorerHandle <T: Explorer> {
    explorer: Option<T>, // Example implementation defined in example_explorer.rs
    current_planet: ID,
    thread_handle: Option<thread::JoinHandle<()>>,
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


impl<T: Explorer + Send + 'static> Orchestrator<T> {
    pub fn new(
        mode: OrchestratorMode,
        planets: HashMap<ID, PlanetHandle>,
        explorers: HashMap<ID, ExplorerHandle<T>>) -> Self {
        Orchestrator {
            time: 0,
            mode,
            planets,
            explorers,
        }
    }

    pub fn run(&mut self) {
        self.initialize();
        while !self.is_game_over() {
            self.execute_cycle();
        }
    }

    fn initialize(&mut self) {
        self.add_planets().unwrap_or_else(|e| {
            log::error!("Failed to add planets: {}", e);
            panic!("Failed to add planets");
        });
        self.fully_connect_planets();
        self.start_planets();
        self.start_explorers();
    }

    fn is_game_over(&self) -> bool {
        self.planets.is_empty()
    }

    fn fully_connect_planets(&mut self) {
        let planet_ids: Vec<_> = self.planets.keys().cloned().collect();
        for i in 0..planet_ids.len() {
            for j in 0..planet_ids.len() {
                if i != j {
                    self.connect_planets(planet_ids[i], planet_ids[j]).unwrap();
                }
            }
        }
    }

    fn start_planets(&mut self) {
        for (id, planet_handle) in self.planets.iter_mut() {
            let mut planet = planet_handle.planet.take().unwrap_or_else(||{
                error!("Planet not found when starting planet thread");
                panic!("Planet not found when starting planet thread");
            });
            let id = *id;
            planet_handle.thread_handle = Some(thread::spawn(move || {
                planet.run().unwrap_or_else(|e| {
                    log::error!("Planet {} thread terminated with error: {}", id, e);
                });
            }));
        }
    }

    fn start_explorers(&mut self) {
        for (id, explorer_handle) in self.explorers.iter_mut() {
            let mut explorer = explorer_handle.explorer.take().unwrap_or_else(||{
                error!("Explorer not found when starting explorer thread");
                panic!("Explorer not found when starting explorer thread");
            });
            let id = *id;
            explorer_handle.thread_handle = Some(thread::spawn(move || {
                explorer.run().unwrap_or_else(|e| {
                    log::error!("Explorer {} thread terminated with error: {}", id, e);
                });
            }));
        }
    }

    fn execute_cycle(&mut self) {
        todo!()
        // Send sunray and asteroid
        // ...
        // self.time += 1;
    }

    fn get_asteroid_p(&self) -> f32 {
        // A sigmoid function that starts with y=initial_asteroid_probability
        let p_start = AppConfig::get().initial_asteroid_probability;
        let probability = AppConfig::get().asteroid_probability;
        let t0 = (1.0 / probability) * ((1.0 - p_start) / p_start).ln();
        1.0 / (1.0 + (-probability * (self.time as f32 - t0)).exp())
    }

    fn get_sunray_p(&self) -> f32 {
        AppConfig::get().sunray_probability
    }
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
    use crate::orchestrator::example_explorer::ExampleExplorer;

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
use std::collections::HashMap;
use std::thread;

use common_game::components::planet::Planet;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::app::AppConfig;
use crate::orchestrator::BagContent;
use crate::orchestrator::galaxy::Galaxy;
use crate::orchestrator::{ExplorerBuilder, GalaxyBuilder};

/// The Orchestrator is the main entity that manages the game.
/// It's responsible for managing the communication and threads (IPC)
#[allow(dead_code)]
pub(crate) struct Orchestrator {
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    time: u32,

    // Auto/manual
    mode: OrchestratorMode,

    galaxy: Galaxy,

    // List of explorers
    explorers: HashMap<ID, ExplorerHandle>,
    // List of planets
    planets: HashMap<ID, PlanetHandle>,
}

pub(crate) enum OrchestratorMode {
    Auto,
    Manual,
}

// struct used to handle the list of planets.
// This is partly redundant as ID is stored twice,
// But the alternative is to store topology in a separate
// struct which would also require ID as key
// Can be changed if you find a better way
pub(crate) struct PlanetHandle {
    pub thread_handle: thread::JoinHandle<()>,
    pub tx: Sender<OrchestratorToPlanet>,
    pub rx: Receiver<PlanetToOrchestrator>,
    pub tx_explorer: Sender<ExplorerToPlanet>, // Passed to explorers to communicate with the planet
}

// Struct to hold explorers;
// Again ID is probably also in the explorer struct,
// As well as the state. Created explorer trait.
pub(crate) struct ExplorerHandle {
    pub current_planet: ID,
    pub thread_handle: thread::JoinHandle<()>,
    pub tx: Sender<OrchestratorToExplorer>,
    pub rx: Receiver<ExplorerToOrchestrator<BagContent>>,
    pub tx_planet: Sender<PlanetToExplorer>, // Passed to planets to communicate with the explorer
    pub state: ExplorerState,
}

pub enum ExplorerState {
    Autonomous,
    Manual,
    Stopped,
    Destroyed,
}

impl Orchestrator {
    pub fn new(
        mode: OrchestratorMode,
        n_planets: usize,
        explorer_builders: Vec<Box<dyn ExplorerBuilder>>,
    ) -> Result<Self, String> {
        let initial_galaxy = GalaxyBuilder::new()
            .with_fully_connected_topology()
            .with_n_planets(n_planets)
            .with_explorers(explorer_builders)
            .build()?;

        let planets = initial_galaxy
            .planet_inits
            .into_iter()
            .map(|(id, planet_init)| {
                (
                    id,
                    PlanetHandle {
                        thread_handle: Self::start_planet(planet_init.planet, id),
                        tx: planet_init.orchestrator_to_planet_tx,
                        rx: planet_init.planet_to_orchestrator_rx,
                        tx_explorer: planet_init.explorer_to_planet_tx,
                    },
                )
            })
            .collect();

        let explorers = initial_galaxy
            .explorer_inits
            .into_iter()
            .map(|(id, explorer_init)| {
                (
                    id,
                    ExplorerHandle {
                        current_planet: explorer_init.initial_planet,
                        thread_handle: Self::start_explorers(explorer_init.explorer, id),
                        tx: explorer_init.orchestrator_to_explorer_tx,
                        rx: explorer_init.explorer_to_orchestrator_rx,
                        tx_planet: explorer_init.planet_to_explorer_tx,
                        state: ExplorerState::Autonomous,
                    },
                )
            })
            .collect();

        Ok(Orchestrator {
            time: 0,
            mode,
            galaxy: initial_galaxy.galaxy,
            planets,
            explorers,
        })
    }

    pub fn run(&mut self) {
        while !self.is_game_over() {
            self.execute_cycle();
        }
    }

    fn is_game_over(&self) -> bool {
        self.galaxy.get_planets().is_empty()
    }

    fn start_planet(mut planet: Planet, id: ID) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            planet.run().unwrap_or_else(|e| {
                log::error!("Planet {id} thread terminated with error: {e}");
            });
        })
    }

    fn start_explorers(explorer: Box<dyn ExplorerBuilder>, id: ID) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let mut explorer_instance = explorer.build().unwrap_or_else(|e| {
                panic!("Failed to build explorer {id}: {e}");
            });
            explorer_instance.run().unwrap_or_else(|e| {
                log::error!("Explorer {id} thread terminated with error: {e}");
            });
        })
    }

    fn execute_cycle(&mut self) {
        todo!()
        // Send sunray and asteroid
        // ...
        // self.time += 1;
    }

    fn handle_planet_destroyed(&mut self, planet_id: ID) {
        self.galaxy.remove_planet(planet_id);

        let handle = self.planets.remove(&planet_id);
        if let Some(planet_handle) = handle {
            planet_handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for destroyed planet {planet_id}: {e:?}");
            });
        }
        let explorers_to_remove: Vec<ID> = self
            .explorers
            .iter()
            .filter_map(|(&explorer_id, explorer_handle)| {
                if explorer_handle.current_planet == planet_id {
                    Some(explorer_id)
                } else {
                    None
                }
            })
            .collect();

        for explorer_id in explorers_to_remove {
            // Unwrap is safe since the explorer cannot be already removed (the ID comes from the planet)
            let handle = self.explorers.remove(&explorer_id).unwrap();
            handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for destroyed explorer {explorer_id}: {e:?}");
            });
        }
    }

    #[allow(clippy::cast_precision_loss)] // f32 is precise enough for our needs
    fn get_asteroid_p(&self) -> f32 {
        // A sigmoid function that starts with y=initial_asteroid_probability
        let p_start = AppConfig::get().initial_asteroid_probability;
        let probability = AppConfig::get().asteroid_probability;
        let t0 = (1.0 / probability) * ((1.0 - p_start) / p_start).ln();
        1.0 / (1.0 + (-probability * (self.time as f32 - t0)).exp())
    }

    #[allow(clippy::unused_self)] // to keep api consistent with get_asteroid_p
    fn get_sunray_p(&self) -> f32 {
        AppConfig::get().sunray_probability
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Orchestrator::new(OrchestratorMode::Auto, 0, vec![]).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_galaxy_create() {
        let orchestrator = Orchestrator::new(OrchestratorMode::Auto, 0, vec![]);
        assert!(orchestrator.is_ok());
    }

    #[test]
    fn test_game_over_empty_galaxy() {
        let orchestrator = Orchestrator::new(OrchestratorMode::Auto, 0, vec![]).unwrap();
        assert!(orchestrator.is_game_over());
    }

    #[test]
    fn verify_probabilities() {
        // verify the initial value and that the probability tends to 1
        let mut orchestrator = Orchestrator::default();
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

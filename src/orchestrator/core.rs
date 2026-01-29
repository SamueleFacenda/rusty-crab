use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use common_game::components::asteroid::Asteroid;
use common_game::components::planet::Planet;
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Sender};

use crate::app::AppConfig;
use crate::orchestrator::{ExplorerLoggingReceiver, ExplorerLoggingSender, PlanetLoggingReceiver, PlanetLoggingSender};
use crate::orchestrator::{ExplorerBuilder, GalaxyBuilder, OrchestratorState, ExplorerHandle, PlanetHandle, ExplorerState};

/// The Orchestrator is the main entity that manages the game.
/// It's responsible for managing the communication and threads (IPC)
#[allow(dead_code)]
pub(crate) struct Orchestrator {
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    time: u32,

    // Auto/manual
    mode: OrchestratorMode,

    state: OrchestratorState,
}

pub(crate) enum OrchestratorMode {
    Auto,
    Manual,
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

        let planets = initial_galaxy.planet_inits.into_iter()
            .map(|(id, planet_init)| {
                (
                    id,
                    PlanetHandle {
                        thread_handle: Self::start_planet(planet_init.planet, id),
                        tx: PlanetLoggingSender::new(planet_init.orchestrator_to_planet_tx),
                        tx_explorer: planet_init.explorer_to_planet_tx,
                    },
                )
            })
            .collect();

        let explorers = initial_galaxy.explorer_inits.into_iter()
            .map(|(id, explorer_init)| {
                (
                    id,
                    ExplorerHandle {
                        current_planet: explorer_init.initial_planet,
                        thread_handle: Self::start_explorers(explorer_init.explorer, id),
                        tx: ExplorerLoggingSender::new(explorer_init.orchestrator_to_explorer_tx),
                        tx_planet: explorer_init.planet_to_explorer_tx,
                        state: ExplorerState::Autonomous,
                    },
                )
            })
            .collect();

        Ok(Orchestrator {
            time: 0,
            mode,
            state: OrchestratorState {
                galaxy: initial_galaxy.galaxy,
                planets,
                explorers,
                planets_rx: PlanetLoggingReceiver::new(initial_galaxy.planet_to_orchestrator_rx),
                explorers_rx: ExplorerLoggingReceiver::new(initial_galaxy.explorer_to_orchestrator_rx),
            }
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        while !self.is_game_over() {
            self.execute_cycle()?;
            self.time += 1;
        }
        Ok(())
    }

    fn is_game_over(&self) -> bool {
        self.state.galaxy.get_planets().is_empty()
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

    fn execute_cycle(&mut self) -> Result<(), String> {
        self.send_sunrays()?;
        self.send_asteroids()?;
        self.send_bag_content_requests()?;

        Ok(())
    }

    fn send_asteroids(&mut self) -> Result<(), String> {
        for planet_id in self.state.galaxy.get_planets() {
            if rand::random::<f32>() < self.get_asteroid_p() {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Asteroid(Asteroid::default()))? {
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: Some(_) } => {}, // planet defended itself
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: None } => { // handle destroyed planet
                        self.state.handle_planet_destroyed(planet_id);
                    },
                    _ => return Err(format!("Unexpected response from planet {planet_id} to asteroid (invalid state)")),
                }
            }
        }
        Ok(())
    }

    fn send_sunrays(&mut self) -> Result<(), String> {
        for planet_id in self.state.galaxy.get_planets() {
            if rand::random::<f32>() < self.get_sunray_p() {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Sunray(Sunray::default()))? {
                    PlanetToOrchestrator::SunrayAck{planet_id} => {}, // planet handled sunray
                    _ => return Err(format!("Unexpected response from planet {planet_id} to sunray (invalid state)")),
                }
            }
        }
        Ok(())
    }

    fn planet_syn_ack(&self, planet_id: ID, msg: OrchestratorToPlanet) -> Result<PlanetToOrchestrator, String> {
        self.state.planets[&planet_id]
            .tx
            .send(msg, planet_id)
            .map_err(|e| e.to_string())?;
        self.state.planets_rx
            .recv_timeout(AppConfig::get().max_wait_time_ms, planet_id)
            .map(|r| if r.planet_id() == planet_id {
                Ok(r)
            } else {
                Err(format!("The wrong planet responded! Expected {planet_id}, got {}", r.planet_id()))
            } )
            .map_err(|e| e.to_string())? // The ? is because it's a nested Result
    }

    fn send_bag_content_requests(&self) -> Result<(), String> {
        for (id, explorer_handle) in &self.state.explorers {
            explorer_handle
                .tx
                .send(OrchestratorToExplorer::BagContentRequest, *id)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
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

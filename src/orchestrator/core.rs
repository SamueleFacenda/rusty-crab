use std::thread;
use common_game::components::planet::Planet;
use common_game::utils::ID;

use crate::orchestrator::{ExplorerLoggingReceiver, ExplorerLoggingSender, OrchestratorUpdateFactory, PlanetLoggingReceiver, PlanetLoggingSender};
use crate::orchestrator::{ExplorerBuilder, GalaxyBuilder, OrchestratorState, ExplorerHandle, PlanetHandle, ExplorerState};
use crate::orchestrator::channel_demultiplexer::{ExplorerChannelDemultiplexer, PlanetChannelDemultiplexer};

/// The Orchestrator is the main entity that manages the game.
/// It's responsible for managing the communication and threads (IPC)
#[allow(dead_code)]
pub(crate) struct Orchestrator {
    // Auto/manual
    mode: OrchestratorMode,

    state: OrchestratorState,
}

#[derive(Clone, Copy)]
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
            mode,
            state: OrchestratorState {
                time: 0,
                galaxy: initial_galaxy.galaxy,
                planets,
                explorers,
                planets_rx: PlanetChannelDemultiplexer::new(
                    PlanetLoggingReceiver::new(initial_galaxy.planet_to_orchestrator_rx)),
                explorers_rx: ExplorerChannelDemultiplexer::new(
                    ExplorerLoggingReceiver::new(initial_galaxy.explorer_to_orchestrator_rx)),
            }
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut update_strategy = OrchestratorUpdateFactory::get_strategy(self.mode);

        while !self.is_game_over() {
            update_strategy.update(&mut self.state)?;
            self.state.time += 1;
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
}

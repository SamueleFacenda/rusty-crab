use std::thread;

use common_game::components::planet::Planet;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestratorKind, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::utils::ID;

use crate::explorers::ExplorerBuilder;
use crate::gui::GuiEventBuffer;
use crate::orchestrator::CommunicationCenter;
use crate::orchestrator::{ExplorerChannelDemultiplexer, PlanetChannelDemultiplexer};
use crate::orchestrator::{
    ExplorerHandle, ExplorerState, GalaxyBuilder, OrchestratorState, PlanetHandle,
};
use crate::orchestrator::{
    ExplorerLoggingReceiver, ExplorerLoggingSender, OrchestratorUpdateFactory,
    PlanetLoggingReceiver, PlanetLoggingSender,
};

/// The Orchestrator is the main entity that manages the game.
/// It's responsible for managing the communication and threads (IPC)
#[allow(dead_code)]
pub(crate) struct Orchestrator {
    // Auto/manual
    mode: OrchestratorMode,

    state: OrchestratorState,
}

#[allow(dead_code)] // only one at a time is used
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

        let (planet_handles, planet_senders) = initial_galaxy
            .planet_inits
            .into_iter()
            .map(|(id, planet_init)| {
                (
                    (
                        id,
                        PlanetHandle {
                            thread_handle: Self::start_planet(planet_init.planet, id),
                            tx_explorer: planet_init.explorer_to_planet_tx,
                        },
                    ),
                    (
                        id,
                        PlanetLoggingSender::new(planet_init.orchestrator_to_planet_tx),
                    ),
                )
            })
            .unzip();

        let (explorer_handles, explorer_senders) = initial_galaxy
            .explorer_inits
            .into_iter()
            .map(|(id, explorer_init)| {
                (
                    (
                        id,
                        ExplorerHandle {
                            current_planet: explorer_init.initial_planet,
                            thread_handle: Self::start_explorers(explorer_init.explorer, id),
                            tx_planet: explorer_init.planet_to_explorer_tx,
                            state: ExplorerState::Autonomous,
                        },
                    ),
                    (
                        id,
                        ExplorerLoggingSender::new(explorer_init.orchestrator_to_explorer_tx),
                    ),
                )
            })
            .unzip();

        Ok(Orchestrator {
            mode,
            state: OrchestratorState {
                time: 0,
                galaxy: initial_galaxy.galaxy,
                planets: planet_handles,
                explorers: explorer_handles,
                communication_center: CommunicationCenter::new(
                    explorer_senders,
                    planet_senders,
                    PlanetChannelDemultiplexer::new(PlanetLoggingReceiver::new(
                        initial_galaxy.planet_to_orchestrator_rx,
                    )),
                    ExplorerChannelDemultiplexer::new(ExplorerLoggingReceiver::new(
                        initial_galaxy.explorer_to_orchestrator_rx,
                    )),
                ),
                gui_events_buffer: GuiEventBuffer::new(),
            },
        })
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.manual_init()?;

        while !self.is_game_over() {
            self.manual_step()?;
        }
        Ok(())
    }
    
    pub fn manual_init(&mut self) -> Result<(), String> {
        self.send_planet_ai_start()?;
        self.send_explorer_ai_start()?;
        Ok(())
    }
    
    pub fn manual_step(&mut self) -> Result<(), String> {
        let mut update_strategy = OrchestratorUpdateFactory::get_strategy(OrchestratorMode::Manual);
        update_strategy.update(&mut self.state)?;
        self.state.time += 1;
        log::info!("--- Time step {} completed ---", self.state.time);
        Ok(())
    }

    pub fn is_game_over(&self) -> bool {
        self.state.galaxy.get_planets().is_empty()
    }
    
    pub fn get_gui_events_buffer(&mut self) -> &mut GuiEventBuffer {
        &mut self.state.gui_events_buffer
    }
    
    pub fn set_mode_auto(&mut self) {
        self.mode = OrchestratorMode::Auto;
    }
    
    pub fn set_mode_manual(&mut self) {
        self.mode = OrchestratorMode::Manual;
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
                log::error!("Failed to build explorer {id}: {e}");
                panic!("Failed to build explorer {id}: {e}");
            });
            explorer_instance.run().unwrap_or_else(|e| {
                log::error!("Explorer {id} thread terminated with error: {e}");
            });
        })
    }

    fn send_planet_ai_start(&mut self) -> Result<(), String> {
        for planet_id in self.state.galaxy.get_planets() {
            self.state.communication_center.planet_req_ack(
                planet_id,
                OrchestratorToPlanet::StartPlanetAI,
                PlanetToOrchestratorKind::StartPlanetAIResult,
            )?;
        }
        Ok(())
    }

    fn send_explorer_ai_start(&mut self) -> Result<(), String> {
        for explorer_id in self.state.explorers.keys() {
            self.state.communication_center.explorer_req_ack(
                *explorer_id,
                OrchestratorToExplorer::StartExplorerAI,
                ExplorerToOrchestratorKind::StartExplorerAIResult,
            )?;
        }
        Ok(())
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

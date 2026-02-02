use std::collections::HashMap;
use std::thread;
use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestratorKind, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::Sender;

use crate::gui::GuiEventBuffer;
use crate::orchestrator::communication::{ExplorerCommunicationCenter, PlanetCommunicationCenter};
use crate::orchestrator::galaxy::Galaxy;

/// struct used to handle the list of planets.
pub(crate) struct PlanetHandle {
    pub thread_handle: thread::JoinHandle<()>,
    pub tx_explorer: Sender<ExplorerToPlanet> // Passed to explorers to communicate with the planet
}

/// Struct to hold explorers handles and state;
pub(crate) struct ExplorerHandle {
    pub current_planet: ID,
    pub thread_handle: thread::JoinHandle<()>,
    pub tx_planet: Sender<PlanetToExplorer> // Passed to planets to communicate with the explorer
}

/// Struct that holds the state of the orchestrator, with some basic methods to manipulate it.
pub(crate) struct OrchestratorState {
    // The behavior of the orchestrator is defined by turn-like units of time
    // Alternatively can be done real-time, but that's harder to implement
    pub time: u32,

    pub galaxy: Galaxy,

    // List of explorers
    pub explorers: HashMap<ID, ExplorerHandle>,
    // List of planets
    pub planets: HashMap<ID, PlanetHandle>,

    pub planets_communication_center: PlanetCommunicationCenter,
    pub explorers_communication_center: ExplorerCommunicationCenter,

    pub gui_events_buffer: GuiEventBuffer,
}

#[derive(Debug)]
pub(crate) enum OrchestratorManualAction {
    SendSunray { planet_id: ID },
    SendAsteroid { planet_id: ID },
    GenerateBasic { explorer_id: ID, resource: BasicResourceType },
    GenerateComplex { explorer_id: ID, resource: ComplexResourceType },
    MoveExplorer { explorer_id: ID, destination_planet_id: ID },
}

impl OrchestratorState {
    pub fn handle_planet_destroyed(&mut self, planet_id: ID) -> Result<(), String> {
        self.galaxy.remove_planet(planet_id);

        self.kill_planet(planet_id)?;

        let explorers_to_remove = self.get_explorers_on_planet(planet_id);
        for explorer_id in explorers_to_remove {
            self.kill_explorer(explorer_id)?;
        }
        Ok(())
    }

    fn kill_planet(&mut self, planet_id: ID) -> Result<(), String> {
        let handle = self.planets.remove(&planet_id);
        if let Some(planet_handle) = handle {
            self.planets_communication_center.req_ack(
                planet_id,
                OrchestratorToPlanet::KillPlanet,
                PlanetToOrchestratorKind::KillPlanetResult
            )?;

            planet_handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for killed planet {planet_id}: {e:?}");
            });
            self.planets_communication_center.remove(planet_id);
            self.gui_events_buffer.planet_destroyed(planet_id);
        }
        Ok(())
    }

    fn kill_explorer(&mut self, explorer_id: ID) -> Result<(), String> {
        let handle = self.explorers.remove(&explorer_id);
        if let Some(explorer_handle) = handle {
            self.explorers_communication_center.req_ack(
                explorer_id,
                OrchestratorToExplorer::KillExplorer,
                ExplorerToOrchestratorKind::KillExplorerResult
            )?;

            explorer_handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for killed explorer {explorer_id}: {e:?}");
            });
            self.explorers_communication_center.remove(explorer_id);
        }
        Ok(())
    }

    fn get_explorers_on_planet(&self, planet_id: ID) -> Vec<ID> {
        self.explorers
            .iter()
            .filter(|(_, handle)| handle.current_planet == planet_id)
            .map(|(&explorer_id, _)| explorer_id)
            .collect()
    }
}

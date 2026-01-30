use crate::gui::GuiEventBuffer;
use crate::orchestrator::CommunicationCenter;
use crate::orchestrator::galaxy::Galaxy;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::thread;

pub enum ExplorerState {
    Autonomous,
    Manual,
    Stopped,
    Destroyed,
}

/// struct used to handle the list of planets.
pub(crate) struct PlanetHandle {
    pub thread_handle: thread::JoinHandle<()>,
    pub tx_explorer: Sender<ExplorerToPlanet>, // Passed to explorers to communicate with the planet
}

/// Struct to hold explorers handles and state;
pub(crate) struct ExplorerHandle {
    pub current_planet: ID,
    pub thread_handle: thread::JoinHandle<()>,
    pub tx_planet: Sender<PlanetToExplorer>, // Passed to planets to communicate with the explorer
    pub state: ExplorerState,
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

    pub communication_center: CommunicationCenter,

    pub gui_events_buffer: GuiEventBuffer,
}

impl OrchestratorState {
    pub fn handle_planet_destroyed(&mut self, planet_id: ID) -> Result<(), String> {
        self.galaxy.remove_planet(planet_id);

        let handle = self.planets.remove(&planet_id);
        if let Some(planet_handle) = handle {
            self.communication_center.planet_req_ack(
                planet_id,
                OrchestratorToPlanet::KillPlanet,
                PlanetToOrchestratorKind::KillPlanetResult,
            )?;

            planet_handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for destroyed planet {planet_id}: {e:?}");
            });
            self.communication_center.remove_planet(planet_id);
            self.gui_events_buffer.planet_destroyed(planet_id);
        }

        let explorers_to_remove = self.get_explorers_on_planet(planet_id);
        for explorer_id in explorers_to_remove {
            // Unwrap is safe since the explorer cannot be already removed (the ID comes from the planet)
            let handle = self.explorers.remove(&explorer_id).unwrap();
            handle.thread_handle.join().unwrap_or_else(|e| {
                log::error!("Failed to join thread for destroyed explorer {explorer_id}: {e:?}");
            });
            self.communication_center.remove_explorer(explorer_id); // This disconnects the explorer
        }
        Ok(())
    }

    pub fn get_explorers_on_planet(&self, planet_id: ID) -> Vec<ID> {
        self.explorers
            .iter()
            .filter(|(_, handle)| handle.current_planet == planet_id)
            .map(|(&explorer_id, _)| explorer_id)
            .collect()
    }
}

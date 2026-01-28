use std::collections::HashMap;
use common_game::components::planet::Planet;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use common_game::utils::ID;
use crossbeam_channel::{unbounded, Receiver, Sender};
use crate::orchestrator::example_explorer::BagContent;
use crate::orchestrator::Explorer;
use crate::orchestrator::galaxy::Galaxy;
use crate::orchestrator::orchestrator::{ExplorerHandle, PlanetHandle};
use crate::orchestrator::planet_factory::{PlanetFactory, PlanetType};

/// These builders creates all the galaxy entities and their connections
pub(crate) struct GalaxyBuilder {
    fully_connected: bool,
    circular: bool,
    n_planets: usize,
    explorers: Vec<Box<dyn Explorer>>
}

const PLANET_ORDER: [PlanetType; 7] = [
    PlanetType::PanicOutOfOxygen,
    PlanetType::TheCompilerStrikesBack,
    PlanetType::Rustrelli,
    PlanetType::Carbonium,
    PlanetType::OneMillionCrabs,
    PlanetType::HoustonWeHaveABorrow,
    PlanetType::RustEze,
];

struct PlanetInit {
    planet: Planet,
    orchestrator_to_planet_tx: crossbeam_channel::Sender<OrchestratorToPlanet>,
    planet_to_orchestrator_rx: crossbeam_channel::Receiver<PlanetToOrchestrator>,
    explorer_to_planet_tx: crossbeam_channel::Sender<ExplorerToPlanet>,
}

struct ExplorerInit {
    explorer: Box<dyn Explorer>,
    initial_planet: ID,
    explorer_to_orchestrator_tx: crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>,
    explorer_to_orchestrator_rx: crossbeam_channel::Receiver<ExplorerToOrchestrator<BagContent>>,
    orchestrator_to_explorer_tx: crossbeam_channel::Sender<OrchestratorToExplorer>,
    orchestrator_to_explorer_rx: crossbeam_channel::Receiver<OrchestratorToExplorer>,
}

struct GalaxyBuilderResult {
    galaxy: Galaxy,
    planet_inits: HashMap<ID, PlanetInit>,
    explorer_inits: HashMap<ID, ExplorerInit>,
}

impl GalaxyBuilder {
    pub fn new() -> Self {
        GalaxyBuilder {
            fully_connected: false,
            circular: false,
            n_planets: 0,
            explorers: vec![],
        }
    }

    pub fn with_fully_connected_topology(self) -> Self {
        GalaxyBuilder{
            fully_connected: true,
            ..self
        }
    }

    pub fn with_circular_topology(self) -> Self {
        GalaxyBuilder{
            circular: true,
            ..self
        }
    }

    pub fn with_n_planets(self, n: usize) -> Self {
        GalaxyBuilder{
            n_planets: n,
            ..self
        }
    }

    pub fn with_explorers(self, explorers: Vec<Box<dyn Explorer>>) -> Self {
        GalaxyBuilder{
            explorers,
            ..self
        }
    }

    pub fn build(mut self) -> Result<GalaxyBuilderResult, String> {
        if self.fully_connected && self.circular {
            return Err("Cannot have both fully connected and circular topology".to_string());
        }
        if !self.fully_connected && !self.circular {
            return Err("Must specify either fully connected or circular topology".to_string());
        }
        let galaxy = self.get_galaxy()?;
        let planet_inits = self.get_planets_init()?;
        let explorer_inits = self.get_explorer_init();

        Ok(GalaxyBuilderResult {
            galaxy,
            planet_inits,
            explorer_inits,
        })
    }

    fn get_galaxy(&self) -> Result<Galaxy, String> {
        let planet_ids = self.get_planet_ids();
        if self.fully_connected {
            Galaxy::make_fully_connected(planet_ids)
        } else{
            Galaxy::make_circular(planet_ids)
        }
    }

    fn get_explorer_init(&mut self) -> HashMap<ID, ExplorerInit> {
        let explorers = std::mem::take(&mut self.explorers);
        let mut handles = HashMap::new();
        let explorer_ids = self.get_explorer_ids();
        for (explorer, id) in explorers.into_iter().zip(explorer_ids) {
            let ex_to_orch_channel = unbounded();
            let orch_to_ex_channel = unbounded();
            handles.insert(id, ExplorerInit{
                explorer,
                initial_planet: 0, // all explorers start at planet 0
                explorer_to_orchestrator_tx: ex_to_orch_channel.0,
                explorer_to_orchestrator_rx: ex_to_orch_channel.1,
                orchestrator_to_explorer_tx: orch_to_ex_channel.0,
                orchestrator_to_explorer_rx: orch_to_ex_channel.1,
            });
        }
        handles
    }

    fn get_planets_init(&mut self) -> Result<HashMap<ID, PlanetInit>, String> {
        let mut handles = HashMap::new();
        for planet_id in self.get_planet_ids().iter() {
            let orch_to_planet_channel = unbounded();
            let planet_to_orch_channel = unbounded();
            let explorer_to_planet_channel = unbounded();
            handles.insert(*planet_id, PlanetInit{
                planet: GalaxyBuilder::get_planet(
                    *planet_id,
                    planet_to_orch_channel.0,
                    orch_to_planet_channel.1,
                    explorer_to_planet_channel.1,
                )?,
                orchestrator_to_planet_tx: orch_to_planet_channel.0,
                planet_to_orchestrator_rx: planet_to_orch_channel.1,
                explorer_to_planet_tx: explorer_to_planet_channel.0,
            });
        }
        Ok(handles)
    }

    /// Create the planet instance based on its type
    fn get_planet(
        id: ID,
        p_to_o_tx: Sender<PlanetToOrchestrator>,
        o_to_p_rx: Receiver<OrchestratorToPlanet>,
        e_to_p: Receiver<ExplorerToPlanet>
    ) -> Result<Planet, String> {
        let planet_type = PLANET_ORDER[(id as usize) % PLANET_ORDER.len()];
        PlanetFactory::make_planet(
            planet_type,
            id,
            p_to_o_tx,
            o_to_p_rx,
            e_to_p,
        )
    }

    fn get_planet_ids(&self) -> Vec<ID> {
        (0..self.n_planets).map(|i| i as ID).collect()
    }

    fn get_explorer_ids(&self) -> Vec<ID> {
        (self.n_planets..(self.n_planets+self.explorers.len())).map(|i| i as ID).collect()
    }
}

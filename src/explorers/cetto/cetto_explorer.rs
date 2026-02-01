use std::collections::{HashMap, HashSet};
use bevy::prelude::Res;
use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, GenericResource, Oxygen, ResourceType};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::{BagContent, Explorer};


pub struct CettoExplorer {
    id: ID,

    // Current State
    current_planet_id: ID,
    mode: ExplorerMode,

    // Communication Channels
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
    tx_planet: Option<Sender<ExplorerToPlanet>>,
    rx_planet: Receiver<PlanetToExplorer>,

    // Resources
    bag: Bag,

    // Information collection
    knowledge: ExplorerKnowledge,
}

enum ExplorerMode {
    Auto,
    Manual,
    Stopped,
    Killed,
}

struct ExplorerKnowledge {
    galaxy: GalaxyInfo,
    planets: HashMap<ID, PlanetInfo>,
    goal: HashMap<ResourceType, i32>  // All the resources he needs to Mine&Craft
}

impl Default for ExplorerKnowledge {
    fn default() -> Self {
        let mut goal: HashMap<ResourceType, i32> = HashMap::new();
        goal.insert(ResourceType::Basic(BasicResourceType::Oxygen), 3);
        goal.insert(ResourceType::Basic(BasicResourceType::Carbon), 4);
        goal.insert(ResourceType::Basic(BasicResourceType::Hydrogen), 3);
        goal.insert(ResourceType::Basic(BasicResourceType::Silicon), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Life), 2);
        goal.insert(ResourceType::Complex(ComplexResourceType::Water), 3);
        goal.insert(ResourceType::Complex(ComplexResourceType::Robot), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Diamond), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::Dolphin), 1);
        goal.insert(ResourceType::Complex(ComplexResourceType::AIPartner), 1);

        ExplorerKnowledge {
            galaxy: GalaxyInfo::default(),
            planets: HashMap::new(),
            goal
        }
    }
}

impl ExplorerKnowledge {
    fn update_planet(&mut self, id: ID, planet_info: PlanetInfo) {
        self.planets.insert(id, planet_info);
    }

    fn add_connection(&mut self, a: ID, b: ID) {
        self.galaxy.add_connection(a, b);
    }

    fn remove_connection(&mut self, a: ID, b: ID) {
        self.galaxy.remove_connection(&a, &b);
    }
}

struct Bag {
    basic_resources: Vec<BasicResource>,
    complex_resources: Vec<ComplexResource>,
}
struct PlanetInfo {
    basic_type: BasicResourceType,
    complex_type: Some(ComplexResourceType),
    energy_available: i32,
    is_destroyed: bool
}
struct GalaxyInfo{
    connections: HashMap<ID, HashSet<ID>>
}


impl Default for Bag {
    fn default() -> Self {
        Bag {
            basic_resources: vec![],
            complex_resources: vec![]
        }
    }
}

impl PlanetInfo {
    fn new(
        basic_type: BasicResourceType,
        complex_type: ComplexResourceType,
        energy_available: i32
    ) -> PlanetInfo {
        PlanetInfo {
            basic_type,
            complex_type,
            energy_available,
            is_destroyed: false
        }
    }

    fn update(&mut self, energy: i32) {
        self.energy_available = energy;
    }
}

impl Default for GalaxyInfo {
    fn default() -> Self {
        GalaxyInfo {
            connections: HashMap::new()
        }
    }
}

impl GalaxyInfo {
    fn add_connection(&mut self, a: ID, b: ID) {
        self.connections
            .entry(a)
            .or_insert_with(HashSet::new)
            .insert(b);
    }

    // Used when planets get destroyed, it removes the connection both ways
    fn remove_connection(&mut self, a: &ID, b: &ID) {
        if let Some(conns) = self.connections.get_mut(a) {
            conns.remove(b);
        }
        if let Some(conns) = self.connections.get_mut(b) {
            conns.remove(a);
        }
    }
}


impl Explorer for CettoExplorer {
    fn new(
        id: ID,
        current_planet: ID,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagContent>>,
        tx_current_planet: Sender<ExplorerToPlanet>,
        rx_planet: Receiver<PlanetToExplorer>,
    ) -> Self {
        CettoExplorer {
            id,
            current_planet_id: current_planet,
            mode: ExplorerMode::Auto,
            rx_orchestrator,
            tx_orchestrator,
            tx_planet: Some(tx_current_planet),
            rx_planet,
            bag: Bag::default(),
            knowledge: ExplorerKnowledge::default(),
        }
    }

    fn run(&mut self) -> Result<(), String> {
        loop {
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    if let Err(e) = self.handle_orchestrator_message(msg) {
                        log::error!("Error handling orchestrator message: {e}");
                    }
                }
                Err(e) => {
                    log::error!("Error receiving message from orchestrator: {e}");
                    Err(e.to_string())?;
                }
            }
        }
    }

    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> Result<(), String> {
        Ok(())
    }
}

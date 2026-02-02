use std::collections::BTreeMap;

use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

use crate::app::AppConfig;
use crate::explorers::BagContent;
use crate::orchestrator::{Orchestrator, PLANET_ORDER, PlanetType};

// Functions to bridge the orchestrator state with the GUI resources
impl Orchestrator {
    pub fn get_planets_info(&self) -> PlanetInfoMap {
        let mut map = BTreeMap::new();
        // Check all the IDs, this usually is not reliable but the GUI can only have n planets from the config
        for id in 1..=AppConfig::get().number_of_planets {
            match self.get_planet_state(id as ID) {
                Some(Ok(state)) => {
                    map.insert(id, PlanetInfo {
                        status: Status::Running,
                        energy_cells: state.energy_cells,
                        charged_cells_count: state.charged_cells_count,
                        rocket: state.has_rocket,
                        name: PLANET_ORDER[(id as usize - 1) % PLANET_ORDER.len()]
                    });
                }
                Some(Err(e)) => {
                    log::error!("Error getting state for planet {id}: {e}");
                }
                None => {
                    // Planet not found: already destroyed
                    map.insert(id, PlanetInfo {
                        status: Status::Dead,
                        energy_cells: vec![],
                        charged_cells_count: 0,
                        rocket: false,
                        name: PLANET_ORDER[0]
                    });
                }
            }
        }

        PlanetInfoMap { map }
    }

    #[allow(clippy::cast_possible_truncation)] // We will never have that many planets
    pub fn get_explorer_states(&self) -> ExplorerInfoMap {
        let cfg = AppConfig::get();
        let mut map = BTreeMap::new();
        for id in (cfg.number_of_planets + 1)..=(cfg.number_of_planets + cfg.explorers.len() as u32) {
            match self.get_explorer_current_planet(id) {
                Some(current_planet_id) => {
                    let bag = self.get_explorer_bag(id).cloned().unwrap_or_default();
                    map.insert(id, ExplorerInfo { status: Status::Running, current_planet_id, bag });
                }
                None => {
                    // Explorer not found: already dead
                    map.insert(id, ExplorerInfo {
                        status: Status::Dead,
                        current_planet_id: 0,
                        bag: BagContent::default()
                    });
                }
            }
        }
        ExplorerInfoMap { map }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
#[allow(dead_code)] // not used yet by the gui
pub enum Status {
    Running,
    Paused,
    Dead
}

#[derive(PartialEq, Debug, Clone)]
pub struct PlanetInfo {
    pub status: Status,
    pub energy_cells: Vec<bool>,
    pub charged_cells_count: usize,
    pub rocket: bool,
    pub name: PlanetType
}

#[derive(Clone)]
pub struct PlanetInfoMap {
    map: BTreeMap<u32, PlanetInfo>
}

impl PlanetInfoMap {
    pub fn iter(&self) -> impl Iterator<Item = (&u32, &PlanetInfo)> { self.map.iter() }

    pub fn get_info(&self, id: u32) -> Option<PlanetInfo> { self.map.get(&id).cloned() }

    #[allow(clippy::trivially_copy_pass_by_ref)] // defined in gui, not out api
    pub fn get_status(&self, id: &ID) -> Status { self.map.get(id).unwrap().status }
}

#[derive(Debug)]
pub struct ExplorerInfo {
    pub status: Status,
    pub current_planet_id: ID,
    pub bag: BagContent
}

#[derive(Debug)]
pub struct ExplorerInfoMap {
    map: BTreeMap<u32, ExplorerInfo>
}

impl ExplorerInfoMap {
    #[allow(clippy::trivially_copy_pass_by_ref)] // defined in gui, not out api
    pub fn get(&self, id: &u32) -> Option<&ExplorerInfo> { self.map.get(id) }

    #[allow(clippy::trivially_copy_pass_by_ref)] // defined in gui, not out api
    pub fn get_current_planet(&self, id: &u32) -> u32 { self.map.get(id).unwrap().current_planet_id }
}

pub enum OrchestratorEvent {
    PlanetDestroyed { planet_id: u32 },
    SunraySent { planet_id: u32 },
    SunrayReceived { planet_id: u32 },
    AsteroidSent { planet_id: u32 },
    ExplorerMoved { explorer_id: u32, destination: u32 },
    BasicResourceGenerated { explorer_id: u32, resource: BasicResourceType },
    ComplexResourceGenerated { explorer_id: u32, resource: ComplexResourceType }
}

pub fn get_planet_basic_resources(planet_type: PlanetType) -> Vec<BasicResourceType> {
    match planet_type {
        PlanetType::PanicOutOfOxygen | PlanetType::Carbonium | PlanetType::HoustonWeHaveABorrow =>
            vec![BasicResourceType::Carbon],
        PlanetType::TheCompilerStrikesBack => vec![BasicResourceType::Silicon],
        PlanetType::Rustrelli | PlanetType::OneMillionCrabs | PlanetType::RustEze => vec![
            BasicResourceType::Carbon,
            BasicResourceType::Silicon,
            BasicResourceType::Oxygen,
            BasicResourceType::Hydrogen,
        ]
    }
}

pub fn get_planet_complex_resources(planet_type: PlanetType) -> Vec<ComplexResourceType> {
    match planet_type {
        PlanetType::PanicOutOfOxygen => vec![
            ComplexResourceType::Water,
            ComplexResourceType::Life,
            ComplexResourceType::Dolphin,
            ComplexResourceType::Robot,
            ComplexResourceType::Diamond,
            ComplexResourceType::AIPartner,
        ],
        PlanetType::TheCompilerStrikesBack =>
            vec![ComplexResourceType::Robot, ComplexResourceType::AIPartner, ComplexResourceType::Diamond],
        PlanetType::Rustrelli
        | PlanetType::Carbonium
        | PlanetType::OneMillionCrabs
        | PlanetType::HoustonWeHaveABorrow
        | PlanetType::RustEze => vec![]
    }
}

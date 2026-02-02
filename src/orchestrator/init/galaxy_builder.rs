use std::collections::HashMap;

use common_game::components::planet::Planet;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender, unbounded};

use crate::app::AppConfig;
use crate::explorers::{BagContent, ExplorerBuilder};
use crate::orchestrator::{Galaxy, PlanetFactory, PlanetType};

/// This struct creates and initializes all the galaxy entities, with the help of the corresponding
/// factories/builders.
pub(crate) struct GalaxyBuilder {
    fully_connected: bool,
    circular: bool,
    n_planets: u32,
    explorers: Vec<Box<dyn ExplorerBuilder>>,
    explorer_to_orchestrator:
        (Sender<ExplorerToOrchestrator<BagContent>>, Receiver<ExplorerToOrchestrator<BagContent>>),
    planet_to_orchestrator: (Sender<PlanetToOrchestrator>, Receiver<PlanetToOrchestrator>)
}

pub const PLANET_ORDER: [PlanetType; 6] = [
    PlanetType::PanicOutOfOxygen,
    PlanetType::TheCompilerStrikesBack,
    PlanetType::Rustrelli,
    PlanetType::Carbonium,
    // PlanetType::OneMillionCrabs,
    PlanetType::HoustonWeHaveABorrow,
    PlanetType::RustEze
];

// DTOs used to initialize the entities
pub(crate) struct PlanetInit {
    pub planet: Planet,
    pub orchestrator_to_planet_tx: Sender<OrchestratorToPlanet>,
    pub explorer_to_planet_tx: Sender<ExplorerToPlanet>
}

pub(crate) struct ExplorerInit {
    pub explorer: Box<dyn ExplorerBuilder>,
    pub initial_planet: ID,
    pub orchestrator_to_explorer_tx: Sender<OrchestratorToExplorer>,
    pub planet_to_explorer_tx: Sender<PlanetToExplorer>
}

pub(crate) struct GalaxyBuilderResult {
    pub galaxy: Galaxy,
    pub planet_inits: HashMap<ID, PlanetInit>,
    pub explorer_inits: HashMap<ID, ExplorerInit>,
    pub planet_to_orchestrator_rx: crossbeam_channel::Receiver<PlanetToOrchestrator>,
    pub explorer_to_orchestrator_rx: crossbeam_channel::Receiver<ExplorerToOrchestrator<BagContent>>
}

impl GalaxyBuilder {
    pub fn new() -> Self {
        GalaxyBuilder {
            fully_connected: false,
            circular: false,
            n_planets: 0,
            explorers: vec![],
            explorer_to_orchestrator: unbounded(),
            planet_to_orchestrator: unbounded()
        }
    }

    pub fn with_fully_connected_topology(self) -> Self { GalaxyBuilder { fully_connected: true, ..self } }

    #[allow(dead_code)] // not currently used but still useful
    pub fn with_circular_topology(self) -> Self { GalaxyBuilder { circular: true, ..self } }

    pub fn with_n_planets(self, n: u32) -> Self { GalaxyBuilder { n_planets: n, ..self } }

    pub fn with_explorers(self, explorers: Vec<Box<dyn ExplorerBuilder>>) -> Self {
        GalaxyBuilder { explorers, ..self }
    }

    pub fn build(mut self) -> Result<GalaxyBuilderResult, String> {
        if self.fully_connected && self.circular {
            return Err("Cannot have both fully connected and circular topology".to_string());
        }
        if !self.fully_connected && !self.circular {
            return Err("Must specify either fully connected or circular topology".to_string());
        }
        if self.n_planets == 0 && !self.explorers.is_empty() {
            return Err("Cannot have explorers without planets".to_string());
        }

        let galaxy = self.get_galaxy()?;
        let planet_inits = self.get_planets_init()?;
        let explorer_inits = if let Some(first_planet_init) = &planet_inits.get(&AppConfig::get().initial_planet_id) {
            self.get_explorers_init(&first_planet_init.explorer_to_planet_tx)
        } else {
            HashMap::new()
        };

        Ok(GalaxyBuilderResult {
            galaxy,
            planet_inits,
            explorer_inits,
            planet_to_orchestrator_rx: self.planet_to_orchestrator.1,
            explorer_to_orchestrator_rx: self.explorer_to_orchestrator.1
        })
    }

    fn get_galaxy(&self) -> Result<Galaxy, String> {
        let planet_ids = self.get_planet_ids();
        if self.fully_connected {
            Galaxy::make_fully_connected(&planet_ids)
        } else {
            Galaxy::make_circular(&planet_ids)
        }
    }

    fn get_explorers_init(&mut self, first_planet_sender: &Sender<ExplorerToPlanet>) -> HashMap<ID, ExplorerInit> {
        let mut handles = HashMap::new();
        let explorer_ids = self.get_explorer_ids();
        for (explorer, id) in self.explorers.drain(..).zip(explorer_ids) {
            let orch_to_ex_channel = unbounded();
            let plan_to_ex_channel = unbounded();
            let explorer = explorer
                .with_id(id)
                .with_current_planet(AppConfig::get().initial_planet_id)
                .with_orchestrator_rx(orch_to_ex_channel.1)
                .with_orchestrator_tx(self.explorer_to_orchestrator.0.clone())
                .with_planet_rx(plan_to_ex_channel.1)
                .with_current_planet_tx(first_planet_sender.clone());
            handles.insert(id, ExplorerInit {
                explorer,
                initial_planet: AppConfig::get().initial_planet_id,
                orchestrator_to_explorer_tx: orch_to_ex_channel.0,
                planet_to_explorer_tx: plan_to_ex_channel.0
            });
        }
        handles
    }

    fn get_planets_init(&mut self) -> Result<HashMap<ID, PlanetInit>, String> {
        let mut handles = HashMap::new();
        for planet_id in self.get_planet_ids() {
            let orch_to_planet_channel = unbounded();
            let explorer_to_planet_channel = unbounded();
            handles.insert(planet_id, PlanetInit {
                planet: GalaxyBuilder::get_planet(
                    planet_id,
                    self.planet_to_orchestrator.0.clone(),
                    orch_to_planet_channel.1,
                    explorer_to_planet_channel.1
                )?,
                orchestrator_to_planet_tx: orch_to_planet_channel.0,
                explorer_to_planet_tx: explorer_to_planet_channel.0
            });
        }
        Ok(handles)
    }

    /// Create the planet instance based on its ID
    fn get_planet(
        id: ID,
        p_to_o_tx: Sender<PlanetToOrchestrator>,
        o_to_p_rx: Receiver<OrchestratorToPlanet>,
        e_to_p: Receiver<ExplorerToPlanet>
    ) -> Result<Planet, String> {
        let planet_type = PLANET_ORDER[(id as usize) % PLANET_ORDER.len()];
        PlanetFactory::make_planet(planet_type, id, p_to_o_tx, o_to_p_rx, e_to_p)
    }

    #[allow(clippy::cast_possible_truncation)] // We will never have that many planets
    fn get_planet_ids(&self) -> Vec<ID> { (1..=self.n_planets).map(|i| i as ID).collect() }

    #[allow(clippy::cast_possible_truncation)] // We will never have that many planets
    fn get_explorer_ids(&self) -> Vec<ID> {
        (self.n_planets as usize + 1..=(self.n_planets as usize + self.explorers.len())).map(|i| i as ID).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::time::Duration;

    use crossbeam_channel::unbounded;

    use super::*;

    #[test]
    fn test_galaxy_build() {
        let good_gb = GalaxyBuilder::new().with_fully_connected_topology().with_n_planets(5).build();
        let bad_gb =
            GalaxyBuilder::new().with_fully_connected_topology().with_circular_topology().with_n_planets(5).build();

        assert!(good_gb.is_ok());
        assert!(!bad_gb.is_ok());
    }
}

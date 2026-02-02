use std::collections::HashMap;

use common_game::components::resource::ResourceType;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};

/// Simple DTO
#[derive(Debug, Clone, Default)]
pub struct BagContent {
    pub content: HashMap<ResourceType, usize>
}

/// Trait defining the behavior of an Explorer,
pub trait Explorer {
    fn new(
        id: common_game::utils::ID,
        current_planet: common_game::utils::ID,
        rx_orchestrator: crossbeam_channel::Receiver<OrchestratorToExplorer>,
        tx_orchestrator: crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>,
        tx_first_planet: crossbeam_channel::Sender<ExplorerToPlanet>,
        rx_planet: crossbeam_channel::Receiver<common_game::protocols::planet_explorer::PlanetToExplorer>
    ) -> Self
    where
        Self: Sized;

    fn run(&mut self) -> Result<(), String>;
}

pub(crate) trait ExplorerBuilder: Send {
    fn build(self: Box<Self>) -> Result<Box<dyn Explorer>, String>;
    fn with_orchestrator_rx(
        self: Box<Self>,
        rx: crossbeam_channel::Receiver<OrchestratorToExplorer>
    ) -> Box<dyn ExplorerBuilder>;
    fn with_orchestrator_tx(
        self: Box<Self>,
        tx: crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>
    ) -> Box<dyn ExplorerBuilder>;
    fn with_planet_rx(
        self: Box<Self>,
        rx: crossbeam_channel::Receiver<common_game::protocols::planet_explorer::PlanetToExplorer>
    ) -> Box<dyn ExplorerBuilder>;
    fn with_current_planet_tx(
        self: Box<Self>,
        tx: crossbeam_channel::Sender<ExplorerToPlanet>
    ) -> Box<dyn ExplorerBuilder>;
    fn with_id(self: Box<Self>, id: common_game::utils::ID) -> Box<dyn ExplorerBuilder>;
    fn with_current_planet(self: Box<Self>, planet_id: common_game::utils::ID) -> Box<dyn ExplorerBuilder>;
}

pub(crate) struct ExplorerBuilderImpl<T: Explorer> {
    rx_orchestrator: Option<crossbeam_channel::Receiver<OrchestratorToExplorer>>,
    tx_orchestrator: Option<crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>>,
    rx_planet: Option<crossbeam_channel::Receiver<PlanetToExplorer>>,
    tx_current_planet: Option<crossbeam_channel::Sender<ExplorerToPlanet>>,
    id: Option<common_game::utils::ID>,
    current_planet: Option<common_game::utils::ID>,
    _phantom: std::marker::PhantomData<T>
}

impl<T: Explorer> ExplorerBuilderImpl<T> {
    pub fn new() -> Self {
        ExplorerBuilderImpl {
            rx_orchestrator: None,
            tx_orchestrator: None,
            rx_planet: None,
            tx_current_planet: None,

            id: None,
            current_planet: None,
            _phantom: std::marker::PhantomData
        }
    }
}

impl<T: Explorer + 'static + Send> ExplorerBuilder for ExplorerBuilderImpl<T> {
    fn build(self: Box<Self>) -> Result<Box<dyn Explorer>, String> {
        if self.id.is_none() {
            return Err("Explorer ID not set".to_string());
        }
        if self.rx_orchestrator.is_none() {
            return Err("Orchestrator RX channel not set".to_string());
        }
        if self.tx_orchestrator.is_none() {
            return Err("Orchestrator TX channel not set".to_string());
        }
        if self.rx_planet.is_none() {
            return Err("Planet RX channel not set".to_string());
        }
        if self.current_planet.is_none() {
            return Err("Current planet ID not set".to_string());
        }
        if self.tx_current_planet.is_none() {
            return Err("Current planet TX channel not set".to_string());
        }
        Ok(Box::new(T::new(
            self.id.unwrap(),
            self.current_planet.unwrap(),
            self.rx_orchestrator.unwrap(),
            self.tx_orchestrator.unwrap(),
            self.tx_current_planet.unwrap(),
            self.rx_planet.unwrap()
        )))
    }

    fn with_orchestrator_rx(
        self: Box<Self>,
        rx: crossbeam_channel::Receiver<OrchestratorToExplorer>
    ) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { rx_orchestrator: Some(rx), ..*self })
    }

    fn with_orchestrator_tx(
        self: Box<Self>,
        tx: crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>
    ) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { tx_orchestrator: Some(tx), ..*self })
    }

    fn with_planet_rx(
        self: Box<Self>,
        rx: crossbeam_channel::Receiver<common_game::protocols::planet_explorer::PlanetToExplorer>
    ) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { rx_planet: Some(rx), ..*self })
    }

    fn with_current_planet_tx(
        self: Box<Self>,
        tx: crossbeam_channel::Sender<ExplorerToPlanet>
    ) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { tx_current_planet: Some(tx), ..*self })
    }

    fn with_id(self: Box<Self>, id: common_game::utils::ID) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { id: Some(id), ..*self })
    }

    fn with_current_planet(self: Box<Self>, planet_id: common_game::utils::ID) -> Box<dyn ExplorerBuilder> {
        Box::new(ExplorerBuilderImpl { current_planet: Some(planet_id), ..*self })
    }
}

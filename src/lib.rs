use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::resource::{Combinator, Generator};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages;
use std::sync::mpsc;

pub struct RustyCrabPlanetAI {
    // Alternatively can be named ust "AI" as in the docs
    // TODO!
}

impl RustyCrabPlanetAI {
    pub fn new() -> RustyCrabPlanetAI {
        RustyCrabPlanetAI {}
    }
}

impl PlanetAI for RustyCrabPlanetAI {
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: messages::OrchestratorToPlanet,
    ) -> Option<messages::PlanetToOrchestrator> {
        None
    }

    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: messages::ExplorerToPlanet,
    ) -> Option<messages::PlanetToExplorer> {
        None
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        None
    }

    fn start(&mut self, state: &PlanetState) {}

    fn stop(&mut self, state: &PlanetState) {}
}

pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 1;
    let ai = RustyCrabPlanetAI::new();
    let gen_rules = vec![/* your recipes */];
    let comb_rules = vec![/* your recipes */];

    Planet::new(
        id,
        PlanetType::C,
        Box::new(ai),
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        rx_explorer,
    )
    .unwrap() // TEMP
}

use common_game::components::planet::{Planet, PlanetType};
use common_game::protocols::messages;
use std::sync::mpsc;

use super::ai::RustyCrabPlanetAI;

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

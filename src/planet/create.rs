use common_game::components::planet::{Planet, PlanetType};
use common_game::components::resource::{BasicResource, BasicResourceType};
use common_game::protocols::messages;
use common_game::components::resource::BasicResourceType::Carbon;
use common_game::components::resource::ComplexResourceType::{AIPartner, Diamond, Dolphin, Life, Robot, Water};

use super::ai::RustyCrabPlanetAI;

#[allow(unused)]
pub fn create_planet(
    rx_orchestrator: crossbeam_channel::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: crossbeam_channel::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: crossbeam_channel::Receiver<messages::ExplorerToPlanet>,
    basic_resource: BasicResourceType
) -> Planet {
    let id = 96;
    let ai = RustyCrabPlanetAI {};
    let gen_rules = vec![basic_resource];
    let comb_rules = vec![Diamond, Water, Life, Robot, Dolphin, AIPartner];

    // Construct the planet and return it
    Planet::new(
        id,
        PlanetType::C,
        Box::new(ai),
        gen_rules,
        comb_rules,
        (rx_orchestrator, tx_orchestrator),
        rx_explorer,
    ).expect("Failed to create the planet")
}
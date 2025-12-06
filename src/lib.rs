use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use common_game::components::planet::{Planet, PlanetAI, PlanetState, PlanetType};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages;
use std::sync::mpsc;
use common_game::components::resource::{Combinator, Generator};
use common_game::components::resource::BasicResourceType::Silicon;
use common_game::components::resource::ComplexResourceType::{AIPartner, Diamond, Dolphin, Life, Robot, Water};

pub struct RustyCrabPlanetAI{ // Alternatively can be named ust "AI" as in the docs
    //TODO!
}

impl RustyCrabPlanetAI{
    pub fn new() -> RustyCrabPlanetAI{
        RustyCrabPlanetAI{}
    }
}
impl PlanetAI for RustyCrabPlanetAI{
    fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: OrchestratorToPlanet) -> Option<PlanetToOrchestrator> {
        todo!()
    }

    fn handle_explorer_msg(&mut self, state: &mut PlanetState, generator: &Generator, combinator: &Combinator, msg: ExplorerToPlanet) -> Option<PlanetToExplorer> {
        todo!()
    }

    fn handle_asteroid(&mut self, state: &mut PlanetState, _generator: &Generator, _combinator: &Combinator) -> Option<Rocket> {
        if !state.has_rocket(){  // if there is no rocket, create it
            let requested_cell = state.full_cell();
            if requested_cell.is_some() {  // constructs rocket only if possible
                let (_, cell_idx) = requested_cell.unwrap();
                state.build_rocket(cell_idx).unwrap();  // Our C type planet supports rockets, no check needed
            }
        }
        state.take_rocket()
    }

    fn start(&mut self, state: &PlanetState) {
        todo!()
    }


    fn stop(&mut self, state: &PlanetState) {
        todo!()
    }
}

pub fn create_planet(
    rx_orchestrator: mpsc::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: mpsc::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: mpsc::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 67;  // todo: choose a more original number
    let ai = RustyCrabPlanetAI {};
    let gen_rules = vec![Silicon];  // todo: choose which one (max. 1)
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
    ).expect("Failed to create the planet")  //todo: change this if they change the common code
}
use common_game::components::planet::{PlanetAI, PlanetState};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use common_game::protocols as protocols;

pub struct RustyCrabPlanet{
    //TODO!
}

impl RustyCrabPlanet{
    pub fn new() -> RustyCrabPlanet{
        RustyCrabPlanet{}
    }
}
impl PlanetAI for RustyCrabPlanet{
    fn handle_orchestrator_msg(&mut self, state: &mut PlanetState, msg: OrchestratorToPlanet) -> Option<PlanetToOrchestrator> {
        todo!()
    }

    fn handle_explorer_msg(&mut self, state: &mut PlanetState, msg: ExplorerToPlanet) -> Option<PlanetToExplorer> {
        todo!()
    }

    fn handle_asteroid(&mut self, state: &mut PlanetState) -> Option<Rocket> {
        todo!()
    }

    fn start(&mut self, state: &PlanetState) {
        todo!()
    }

    fn stop(&mut self) {
        todo!()
    }
}

pub fn create_planet(/*...*/) -> RustyCrabPlanet {
    todo!()
}
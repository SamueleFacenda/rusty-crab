use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use common_game::components::planet::{Planet, PlanetAI, PlanetState};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages;
use crossbeam_channel::{Receiver, Sender};
use common_game::components::resource::{Combinator, Generator};
use common_game::components::resource::BasicResourceType::Silicon;
use common_game::components::resource::ComplexResourceType::{AIPartner, Diamond, Dolphin, Life, Robot, Water};
use common_game::protocols::messages::PlanetToOrchestrator::*;
use common_game::components::planet::PlanetType;


pub struct RustyCrabPlanetAI{ // Alternatively can be named ust "AI" as in the docs
    //TODO!
}

impl RustyCrabPlanetAI{
    pub fn new() -> RustyCrabPlanetAI{
        RustyCrabPlanetAI{}
    }
}
#[allow(unused)]
impl PlanetAI for RustyCrabPlanetAI{
    fn handle_orchestrator_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: OrchestratorToPlanet,
    ) -> Option<PlanetToOrchestrator> {
        match msg {
            // Handle a sunray.
            // If there is no energy cell, recharge one. Then,
            // If there is no rocket, build one;
            // Else, do nothing.
            OrchestratorToPlanet::Sunray(sunray) => {
                // Charge empty cell if available
                if let Some((cell, _)) = state.empty_cell() {
                    cell.charge(sunray);
                }
                
                // Build rocket if none exists and we have a full cell
                if !state.has_rocket() {
                    if let Some((_, index)) = state.full_cell() {
                        let _ = state.build_rocket(index);
                    }
                }
                
                Some(SunrayAck { planet_id: state.id() })
            }
            OrchestratorToPlanet::InternalStateRequest => {
                let dummy_state = state.to_dummy();
                Some(InternalStateResponse { planet_id: state.id(), planet_state: dummy_state })
            }
            OrchestratorToPlanet::KillPlanet =>{
                // Currently nothing more to do if not stopping planet AI
                self.stop(state);
                Some(KillPlanetResult {planet_id: state.id()})
            }
            // According to docs:
                // The following messages will **not** invoke this handler:
                // - [OrchestratorToPlanet::StartPlanetAI] (see [PlanetAI::start])
                // - [OrchestratorToPlanet::StopPlanetAI] (see [PlanetAI::stop])
                // - [OrchestratorToPlanet::Asteroid] (see [PlanetAI::handle_asteroid])
                // - [OrchestratorToPlanet::IncomingExplorerRequest], as this will be handled automatically by the planet
                // - [OrchestratorToPlanet::OutgoingExplorerRequest] (same as previous one)
            // This leaves out only Sunray and InternalStateRequest. 
            // Returning None for these cases, since there is no neutral message
            OrchestratorToPlanet::Asteroid(_) 
            | OrchestratorToPlanet::StartPlanetAI 
            | OrchestratorToPlanet::StopPlanetAI
            | OrchestratorToPlanet::IncomingExplorerRequest { .. }
            | OrchestratorToPlanet::OutgoingExplorerRequest { .. } => { None }
        }
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

#[allow(unused)]
pub fn create_planet(
    rx_orchestrator: Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: Sender<messages::PlanetToOrchestrator>,
    rx_explorer: Receiver<messages::ExplorerToPlanet>,
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
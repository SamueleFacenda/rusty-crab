use common_game::protocols::messages::{ExplorerToPlanet, OrchestratorToPlanet, PlanetToExplorer, PlanetToOrchestrator};
use common_game::components::planet::{Planet, PlanetAI, PlanetState};
use common_game::components::rocket::Rocket;
use common_game::protocols::messages;
use crossbeam_channel;
use common_game::components::resource::{Combinator, Generator};
use common_game::components::resource::BasicResourceType::Carbon;
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



    fn handle_explorer_msg(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        msg: ExplorerToPlanet,
    ) -> Option<PlanetToExplorer> {
        // TODO: add that if the planet is stopped, return PlanetToExplorer::Stopped;

        match msg {
            ExplorerToPlanet::AvailableEnergyCellRequest { .. } => {
                Some(PlanetToExplorer::AvailableEnergyCellResponse { available_cells: 1 })
            },
            ExplorerToPlanet::SupportedResourceRequest { .. } => {
                Some(PlanetToExplorer::SupportedResourceResponse {
                    resource_list: generator.all_available_recipes()
                })
            },
            ExplorerToPlanet::SupportedCombinationRequest { .. } => {
                Some(PlanetToExplorer::SupportedCombinationResponse {
                    combination_list: combinator.all_available_recipes()
                })
            },
            ExplorerToPlanet::GenerateResourceRequest {
                explorer_id, resource
            } => {
                // Check if the planet can produce the requested basic resource, and whether an
                // energy cell is charged. If so generate the requested resource

                // (It is not explained in the docs what to return if the planet can't satisfy the
                // request, like if cells are not charged or if the planet does not produce the
                // requested basic resource. Of course it returns some kind of None, but how?
                // returning None directly or returning a response with a None resource inside?
                // I chose the latter, change if needed)

                let cell_option = state.full_cell();
                let out;
                if !generator.contains(resource) || cell_option.is_none() {
                    out = None;
                } else {
                    let (cell, idx) = cell_option.unwrap();
                    out = Some(generator.make_carbon(cell).unwrap().to_basic());
                };
                Some(PlanetToExplorer::GenerateResourceResponse { resource: out })
            },
            ExplorerToPlanet::CombineResourceRequest {
                explorer_id, msg
            } => {
                
            }
        }
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
    rx_orchestrator: crossbeam_channel::Receiver<messages::OrchestratorToPlanet>,
    tx_orchestrator: crossbeam_channel::Sender<messages::PlanetToOrchestrator>,
    rx_explorer: crossbeam_channel::Receiver<messages::ExplorerToPlanet>,
) -> Planet {
    let id = 96;
    let ai = RustyCrabPlanetAI {};
    let gen_rules = vec![Carbon];  // todo: choose which one (max. 1)
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use common_game::components::asteroid::Asteroid;
    use common_game::components::sunray::Sunray;
    use crossbeam_channel::{unbounded, Receiver, Sender};

    fn get_test_channels() -> (
        (Receiver<OrchestratorToPlanet>, Sender<PlanetToOrchestrator>),
        (Receiver<ExplorerToPlanet>, Sender<PlanetToExplorer>),
        (Sender<OrchestratorToPlanet>, Receiver<PlanetToOrchestrator>),
        (Sender<ExplorerToPlanet>, Receiver<PlanetToExplorer>),
    ) {
        // Channel 1: Orchestrator -> Planet
        let (tx_orch_in, rx_orch_in) = unbounded::<OrchestratorToPlanet>();
        // Channel 2: Planet -> Orchestrator
        let (tx_orch_out, rx_orch_out) = unbounded::<PlanetToOrchestrator>();

        // Channel 3: Explorer -> Planet
        let (tx_expl_in, rx_expl_in) = unbounded::<ExplorerToPlanet>();
        // Channel 4: Planet -> Explorer
        let (tx_expl_out, rx_expl_out) = unbounded::<PlanetToExplorer>();

        (
            (rx_orch_in, tx_orch_out),
            (rx_expl_in, tx_expl_out),
            (tx_orch_in, rx_orch_out),
            (tx_expl_in, rx_expl_out),
        )
    }

    #[test]
    fn test_planet() {
        let (planet_orch_ch, planet_expl_ch, orch_planet_ch, _) = get_test_channels();

        let (rx_from_orch, tx_from_planet_orch) = planet_orch_ch;
        let (rx_from_expl, _) = planet_expl_ch;
        let (tx_to_planet_orch, rx_to_orch) = orch_planet_ch;


        let mut planet = create_planet(rx_from_orch, tx_from_planet_orch, rx_from_expl);

        // Spawn thread to run the planet
        let handle = thread::spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let res = planet.run();
                match res {
                    Ok(_) => {}
                    Err(err) => {
                        dbg!(err);
                    }
                }
            }));
        });

        // I have put all the tests in one function because the common prelude is very long

        // 1. Start AI
        tx_to_planet_orch
            .send(OrchestratorToPlanet::StartPlanetAI)
            .unwrap();
        match rx_to_orch.recv_timeout(Duration::from_millis(50)) {
            Ok(PlanetToOrchestrator::StartPlanetAIResult { .. }) => {}
            _ => panic!("Planet sent incorrect response"),
        }
        thread::sleep(Duration::from_millis(50));

        // 2. Send Sunray
        tx_to_planet_orch
            .send(OrchestratorToPlanet::Sunray(Sunray::default()))
            .unwrap();

        // Expect Ack
        if let Ok(PlanetToOrchestrator::SunrayAck { planet_id, .. }) =
            rx_to_orch.recv_timeout(Duration::from_millis(200))
        {
            assert_eq!(planet_id, 96);
        } else {
            panic!("Did not receive SunrayAck");
        }

        // 3. Send Asteroid (AI should build rocket using the charged cell)
        tx_to_planet_orch
            .send(OrchestratorToPlanet::Asteroid(Asteroid::default()))
            .unwrap();

        // 4. Expect Survival (Ack with Some(Rocket))
        match rx_to_orch.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToOrchestrator::AsteroidAck {
                   planet_id,
                   rocket,
                   ..
               }) => {
                assert_eq!(planet_id, 96);
                assert!(rocket.is_some(), "Planet failed to build rocket!");
            }
            Ok(_) => panic!("Wrong message type"),
            Err(_) => panic!("Timeout waiting for AsteroidAck"),
        }

        // 5. Stop
        tx_to_planet_orch
            .send(OrchestratorToPlanet::StopPlanetAI)
            .unwrap();
        match rx_to_orch.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToOrchestrator::StopPlanetAIResult { .. }) => {}
            _ => panic!("Planet sent incorrect response"),
        }

        // 6. Try to send a request while stopped
        tx_to_planet_orch
            .send(OrchestratorToPlanet::InternalStateRequest)
            .unwrap();
        match rx_to_orch.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToOrchestrator::Stopped { .. }) => {}
            _ => panic!("Planet sent incorrect response"),
        }

        // 7. Kill planet while stopped
        tx_to_planet_orch
            .send(OrchestratorToPlanet::KillPlanet)
            .unwrap();
        match rx_to_orch.recv_timeout(Duration::from_millis(200)) {
            Ok(PlanetToOrchestrator::KillPlanetResult { .. }) => {}
            _ => panic!("Planet sent incorrect response"),
        }

        // should return immediately
        assert!(handle.join().is_ok(), "Planet thread exited with an error");
    }
}

use std::ops::Deref;
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::components::planet::{ PlanetAI, PlanetState, DummyPlanetState};
use common_game::components::rocket::Rocket;
use common_game::components::sunray::Sunray;
use common_game::components::resource::{Combinator, ComplexResource, ComplexResourceRequest, Generator};
use common_game::logging::ActorType::{Planet, SelfActor, Explorer, Orchestrator};
use common_game::logging::Channel::{Trace, Debug, Info, Warning};
use common_game::logging::{LogEvent, Payload, Participant};
use common_game::logging::EventType::{InternalPlanetAction, MessagePlanetToOrchestrator, MessagePlanetToExplorer, MessageOrchestratorToPlanet, MessageExplorerToPlanet};
use common_game::utils::ID;
use crossbeam_channel;

/// The RustyCrab Planet AI, a defensive, reliable and versatile planet.
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
    fn handle_sunray(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
        sunray: Sunray
    ) {
        if let Some((cell, _)) = state.empty_cell() {
            cell.charge(sunray);
        }

        // Build rocket if none exists and we have a full cell
        if !state.has_rocket() {
            LogEvent::new(Some(Participant::new(Planet, state.id())), Some(Participant::new(Orchestrator, 0u32)), InternalPlanetAction, Debug, Payload::from([
                (String::from("Rocket"), String::from("Got a sunray, building a rocket...")),
            ])).emit();
            if let Some((_, index)) = state.full_cell() {
                let _ = state.build_rocket(index);
            }
        }
    }

    fn handle_asteroid(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator,
    ) -> Option<Rocket> {
        LogEvent::new(Some(Participant::new(Planet, state.id())), None, InternalPlanetAction, Debug, Payload::from([
            (String::from("Asteroid"), String::from("Asteroid received, checking for rocket construction.")),
        ])).emit();
        if !state.has_rocket(){  // if there is no rocket, create it
            LogEvent::new(Some(Participant::new(Planet, state.id())), None, InternalPlanetAction, Info, Payload::from([
                (String::from("Asteroid"), String::from("No defense, trying to build rocket on the fly...")),
            ])).emit();
            let requested_cell = state.full_cell();
            if requested_cell.is_some() {  // constructs rocket only if possible
                let (_, cell_idx) = requested_cell.unwrap();
                state.build_rocket(cell_idx).unwrap();  // Our C type planet supports rockets, no check needed
            }
        }
        state.take_rocket()
    }

    fn handle_internal_state_req(
        &mut self,
        state: &mut PlanetState,
        generator: &Generator,
        combinator: &Combinator
    ) -> DummyPlanetState {
        state.to_dummy()
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

                LogEvent::new(Some(Participant::new(Planet, state.id())), Some(Participant::new(Explorer, explorer_id)), MessageExplorerToPlanet, Debug, Payload::from([
                    (String::from("RustyCrab"), String::from("Explorer requested resource generation.")),
                    (String::from("Resource"), format!("{:?}", resource)),
                ])).emit();

                let cell_option = state.full_cell();
                let out;
                if !generator.contains(resource) || cell_option.is_none() {
                    out = None;
                } else {
                    let (cell, idx) = cell_option.unwrap();
                    out = Some(generator.make_carbon(cell).unwrap().to_basic());
                    // TODO: change make_* if we change resource type
                };
                Some(PlanetToExplorer::GenerateResourceResponse { resource: out })
            },
            ExplorerToPlanet::CombineResourceRequest {
                explorer_id, msg
            } => {
                // Planet C supports all the combinations, so there is no need to check manually
                // if a certain complex combination is allowed or not.
                // Also, the methods make_water, make_*, ..., return a error message if the
                // combination is wrong or if there is no energy, so no need to check it.

                LogEvent::new(Some(Participant::new(Planet, state.id())), Some(Participant::new(Explorer, explorer_id)), MessageExplorerToPlanet, Debug, Payload::from([
                    (String::from("RustyCrab"), String::from("Explorer requested resource combination.")),
                    (String::from("Resource"), format!("{:?}", msg)),
                ])).emit();


                let response_content;
                let cell = state.cell_mut(0); // First and only cell
                // The cell can be charged or not, the error is handled by make_water, make_*...

                match msg {
                    ComplexResourceRequest::Water(r1, r2) => {
                        let combination = combinator.make_water(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::Water(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    },
                    ComplexResourceRequest::Diamond(r1, r2) => {
                        let combination = combinator
                            .make_diamond(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::Diamond(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    },
                    ComplexResourceRequest::Life(r1, r2) => {
                        let combination = combinator
                            .make_life(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::Life(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    },
                    ComplexResourceRequest::Robot(r1, r2) => {
                        let combination = combinator
                            .make_robot(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::Robot(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    },
                    ComplexResourceRequest::Dolphin(r1, r2) => {
                        let combination = combinator
                            .make_dolphin(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::Dolphin(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    },
                    ComplexResourceRequest::AIPartner(r1, r2) => {
                        let combination = combinator
                            .make_aipartner(r1, r2, cell);
                        response_content = combination
                            .map(|complex| ComplexResource::AIPartner(complex))
                            .map_err(|(msg, r1, r2)| {
                                (msg, r1.to_generic(), r2.to_generic())
                            });
                    }

                }

                Some(PlanetToExplorer::CombineResourceResponse {complex_response: response_content })

            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::super::create_planet;
    use super::*;
    use std::thread;
    use std::time::Duration;
    use common_game::components::asteroid::Asteroid;
    use common_game::components::resource::BasicResourceType::Carbon;
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


        let mut planet = create_planet(rx_from_orch, tx_from_planet_orch, rx_from_expl, Carbon);

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
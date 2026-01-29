use std::collections::HashSet;
use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::utils::ID;

use crate::app::AppConfig;
use crate::orchestrator::{BagContent, OrchestratorState};
use crate::orchestrator::probability::ProbabilityCalculator;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;

pub(crate) struct AutoUpdateStrategy {
    explorers_not_passed: HashSet<ID> // planets that have not passed the turn yet
}

impl AutoUpdateStrategy {
    pub(crate) fn new() -> Self {
        Self {
            explorers_not_passed: Default::default(),
        }
    }

    fn execute_cycle(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        self.explorers_not_passed = state.galaxy.get_planets().iter().copied().collect();

        self.send_sunrays(state)?;
        self.send_asteroids(state)?;
        self.send_bag_content_requests(state)?;

        while !self.explorers_not_passed.is_empty() {
            self.check_explorers_responses(state)?;
        }
        Ok(())
    }

    fn send_asteroids(&self, state: &mut OrchestratorState) -> Result<(), String> {
        for planet_id in state.galaxy.get_planets() {
            if rand::random::<f32>() < ProbabilityCalculator::get_asteroid_probability(state.time) {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Asteroid(Asteroid::default()), state)? {
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: Some(_) } => {}, // planet defended itself
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: None } => { // handle destroyed planet
                        state.handle_planet_destroyed(planet_id);
                    },
                    other => return Err(format!("Unexpected response from planet {planet_id} to asteroid: {other:?}")),
                }
            }
        }
        Ok(())
    }

    fn send_sunrays(&self, state: &mut OrchestratorState) -> Result<(), String> {
        for planet_id in state.galaxy.get_planets() {
            if rand::random::<f32>() < ProbabilityCalculator::get_sunray_probability(state.time) {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Sunray(Sunray::default()), state)? {
                    PlanetToOrchestrator::SunrayAck{planet_id} => {}, // planet handled sunray
                    other => return Err(format!("Unexpected response from planet {planet_id} to sunray: {other:?}")),
                }
            }
        }
        Ok(())
    }

    fn planet_syn_ack(&self, planet_id: ID, msg: OrchestratorToPlanet, state: &mut OrchestratorState) -> Result<PlanetToOrchestrator, String> {
        state.planets[&planet_id].tx
            .send(msg, planet_id)
            .map_err(|e| e.to_string())?;
        state.planets_rx
            .recv_from(planet_id)
    }

    fn explorer_syn_ack(&self, explorer_id: ID, msg: OrchestratorToExplorer, state: &mut OrchestratorState) -> Result<ExplorerToOrchestrator<BagContent>, String> {
        state.explorers[&explorer_id].tx
            .send(msg, explorer_id)
            .map_err(|e| e.to_string())?;
        state.explorers_rx
            .recv_from(explorer_id)
    }

    fn send_bag_content_requests(&self, state: &mut OrchestratorState) -> Result<(), String> {
        for (id, explorer_handle) in &state.explorers {
            explorer_handle.tx
                .send(OrchestratorToExplorer::BagContentRequest, *id)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn check_explorers_responses(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        // Copy is necessary since the cycle may alter the set, so we copy before iterating
        for explorer_id in self.explorers_not_passed.iter().copied().collect::<Vec<ID>>() {
            let res = state.explorers_rx.recv_from(explorer_id)?;
            self.process_explorer_message(explorer_id, res, state)?;
        }
        Ok(())
    }

    fn process_explorer_message(&mut self, planet_id: ID, response: ExplorerToOrchestrator<BagContent>, state: &mut OrchestratorState) -> Result<(), String> {
        match response {
            ExplorerToOrchestrator::BagContentResponse { explorer_id, bag_content } => {
                log::info!("Received bag content from explorer {planet_id}: {bag_content:?}");
                self.explorers_not_passed.remove(&planet_id);
                Ok(())
            }
            ExplorerToOrchestrator::NeighborsRequest { explorer_id, current_planet_id} => {
                self.handle_neighbours_request(explorer_id, current_planet_id, state)
            }
            ExplorerToOrchestrator::TravelToPlanetRequest { explorer_id, current_planet_id, dst_planet_id} => {
                self.handle_travel_request(explorer_id, current_planet_id, dst_planet_id, state)
            }

            other => {
                Err(format!("Unexpected response from explorer {planet_id}: {other:?}"))
            }
        }
    }

    fn handle_neighbours_request(&self, explorer_id: ID, current_planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        if current_planet_id != state.explorers[&explorer_id].current_planet {
            return Err(format!("Explorer {explorer_id} requested neighbors for planet {current_planet_id}, but is currently on planet {}", state.explorers[&explorer_id].current_planet));
        }

        let neighbors = state.galaxy.get_planet_neighbours(current_planet_id);
        state.explorers[&explorer_id].tx
            .send(OrchestratorToExplorer::NeighborsResponse { neighbors }, explorer_id)
            .map_err(|e| e.to_string())
    }

    fn handle_travel_request(&self, explorer_id: ID, current_planet_id: ID, dst_planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        if current_planet_id != state.explorers[&explorer_id].current_planet {
            return Err(format!("Explorer {explorer_id} requested travel from planet {current_planet_id}, but is currently on planet {}", state.explorers[&explorer_id].current_planet));
        }

        // Communicate invalid travel if planets are not connected
        if !state.galaxy.are_planets_connected(current_planet_id, dst_planet_id) {
            return self.notify_explorer_invalid_movement(explorer_id, current_planet_id, state);
        }

        self.notify_planet_incoming_explorer(explorer_id, dst_planet_id, state)?;
        self.notify_planet_explorer_left(explorer_id, current_planet_id, state)?;
        self.notify_explorer_successful_movement(explorer_id, dst_planet_id, state)?;

        // Update internal state
        state.explorers.get_mut(&explorer_id).unwrap().current_planet = dst_planet_id;

        Ok(())
    }

    fn notify_explorer_invalid_movement(&self, explorer_id: ID, current_planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        match self.explorer_syn_ack(explorer_id, OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet: None, planet_id: current_planet_id}, state)? {
            ExplorerToOrchestrator::MovedToPlanetResult { explorer_id, planet_id } => {
                if planet_id != current_planet_id {
                    return Err(format!("Explorer {explorer_id} moved to planet {planet_id}, but was expected to stay on planet {current_planet_id}"));
                }
                Ok(())
            },
            other => Err(format!("Unexpected response from explorer {explorer_id} to invalid travel request: {other:?}")),
        }
    }

    fn notify_planet_incoming_explorer(&self, explorer_id: ID, dst_planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        let new_sender = state.explorers[&explorer_id].tx_planet.clone();
        match self.planet_syn_ack(dst_planet_id, OrchestratorToPlanet::IncomingExplorerRequest { explorer_id, new_sender }, state)? {
            PlanetToOrchestrator::IncomingExplorerResponse { planet_id, explorer_id: accepted_explorer_id, res: Ok(()), } => {
                if accepted_explorer_id != explorer_id {
                    return Err(format!("Planet {dst_planet_id} accepted incoming explorer {accepted_explorer_id}, but was expected to accept explorer {explorer_id}"));
                }
                Ok(())
            },
            PlanetToOrchestrator::IncomingExplorerResponse { planet_id, explorer_id, res: Err(e), } => {
                Err(format!("Planet {dst_planet_id} failed to accept incoming explorer {explorer_id}: {e}"))
            },
            other => Err(format!("Unexpected response from planet {dst_planet_id} to incoming explorer {explorer_id}: {other:?}"))
        }
    }

    fn notify_planet_explorer_left(&self, explorer_id: ID, current_planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        match self.planet_syn_ack(current_planet_id, OrchestratorToPlanet::OutgoingExplorerRequest { explorer_id }, state)? {
            PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, explorer_id: left_explorer_id, res: Ok(()), } => {
                if left_explorer_id != explorer_id {
                    return Err(format!("Planet {current_planet_id} confirmed outgoing explorer {left_explorer_id}, but was expected to confirm explorer {explorer_id}"));
                }
                Ok(())
            },
            PlanetToOrchestrator::OutgoingExplorerResponse { planet_id, explorer_id, res: Err(e), } => {
                Err(format!("Planet {current_planet_id} failed to confirm outgoing explorer {explorer_id}: {e}"))
            },
            other => Err(format!("Unexpected response from planet {current_planet_id} to outgoing explorer {explorer_id}: {other:?}"))
        }
    }

    fn notify_explorer_successful_movement(&self, explorer_id: ID, planet_id: ID, state: &mut OrchestratorState) -> Result<(), String> {
        let sender_to_new_planet = Some(state.planets[&planet_id].tx_explorer.clone());
        match self.explorer_syn_ack(explorer_id, OrchestratorToExplorer::MoveToPlanet { sender_to_new_planet, planet_id}, state)? {
            ExplorerToOrchestrator::MovedToPlanetResult { explorer_id, planet_id: new_planet_id } => {
                if planet_id != new_planet_id {
                    return Err(format!("Explorer {explorer_id} moved to planet {new_planet_id}, but was expected to move to planet {planet_id}"));
                }
                Ok(())
            },
            other => return Err(format!("Unexpected response from explorer {explorer_id} to successful travel request: {other:?}")),
        }
    }
}

impl OrchestratorUpdateStrategy for AutoUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        self.execute_cycle(state)
    }
}
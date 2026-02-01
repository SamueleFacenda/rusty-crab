use std::collections::HashSet;

use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, ExplorerToOrchestratorKind, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestratorKind};
use common_game::utils::ID;

use crate::explorers::BagContent;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;
use crate::orchestrator::{OrchestratorState, ProbabilityCalculator};

pub(crate) struct AutoUpdateStrategy<'a> {
    explorers_not_passed: HashSet<ID>, // explorers that have not passed the turn yet
    state: &'a mut OrchestratorState,
}

impl AutoUpdateStrategy<'_> {
    pub(crate) fn new(state: &'_ mut OrchestratorState) -> AutoUpdateStrategy<'_> {
        AutoUpdateStrategy {
            explorers_not_passed: HashSet::default(),
            state,
        }
    }

    fn execute_cycle(&mut self) -> Result<(), String> {
        self.send_sunrays()?;
        self.send_asteroids()?;

        self.explorers_not_passed = self.state.explorers.keys().copied().collect();
        self.send_bag_content_requests()?;

        while !self.explorers_not_passed.is_empty() {
            self.check_explorers_responses()?;
        }
        Ok(())
    }

    fn send_asteroids(&mut self) -> Result<(), String> {
        for planet_id in self.state.galaxy.get_planets() {
            if rand::random::<f32>()
                < ProbabilityCalculator::get_asteroid_probability(self.state.time)
            {
                self.state.gui_events_buffer.asteroid_sent(planet_id);
                let rocket = self
                    .state
                    .planets_communication_center
                    .req_ack(
                        planet_id,
                        OrchestratorToPlanet::Asteroid(Asteroid::default()),
                        PlanetToOrchestratorKind::AsteroidAck,
                    )?
                    .into_asteroid_ack()
                    .unwrap()
                    .1; // Unwrap is safe due to expected kind

                if rocket.is_none() {
                    self.state.handle_planet_destroyed(planet_id)?;
                }
            }
        }
        Ok(())
    }

    fn send_sunrays(&mut self) -> Result<(), String> {
        for planet_id in self.state.galaxy.get_planets() {
            if rand::random::<f32>()
                < ProbabilityCalculator::get_sunray_probability(self.state.time)
            {
                self.state.gui_events_buffer.sunray_sent(planet_id);
                self.state.planets_communication_center.req_ack(
                    planet_id,
                    OrchestratorToPlanet::Sunray(Sunray::default()),
                    PlanetToOrchestratorKind::SunrayAck,
                )?;
                self.state.gui_events_buffer.sunray_received(planet_id);
            }
        }
        Ok(())
    }

    fn send_bag_content_requests(&self) -> Result<(), String> {
        for id in self.state.explorers.keys() {
            self.state
                .explorers_communication_center
                .send_to(*id, OrchestratorToExplorer::BagContentRequest)?;
        }
        Ok(())
    }

    fn check_explorers_responses(&mut self) -> Result<(), String> {
        // Copy is necessary since the cycle may alter the set, so we copy before iterating
        for explorer_id in self
            .explorers_not_passed
            .iter()
            .copied()
            .collect::<Vec<ID>>()
        {
            let res = self
                .state
                .explorers_communication_center
                .recv_from(explorer_id)?;
            self.process_explorer_message(explorer_id, res)?;
        }
        Ok(())
    }

    fn process_explorer_message(
        &mut self,
        planet_id: ID,
        response: ExplorerToOrchestrator<BagContent>,
    ) -> Result<(), String> {
        match response {
            ExplorerToOrchestrator::BagContentResponse {
                explorer_id: _explorer_id,
                bag_content,
            } => {
                log::info!("Received bag content from explorer {planet_id}: {bag_content:?}");
                self.explorers_not_passed.remove(&planet_id);
                Ok(())
            }
            ExplorerToOrchestrator::NeighborsRequest {
                explorer_id,
                current_planet_id,
            } => self.handle_neighbours_request(explorer_id, current_planet_id),
            ExplorerToOrchestrator::TravelToPlanetRequest {
                explorer_id,
                current_planet_id,
                dst_planet_id,
            } => self.handle_travel_request(explorer_id, current_planet_id, dst_planet_id),

            other => Err(format!(
                "Unexpected response from explorer {planet_id}: {other:?}"
            )),
        }
    }

    fn handle_neighbours_request(
        &self,
        explorer_id: ID,
        current_planet_id: ID,
    ) -> Result<(), String> {
        if current_planet_id != self.state.explorers[&explorer_id].current_planet {
            return Err(format!(
                "Explorer {explorer_id} requested neighbors for planet {current_planet_id}, but is currently on planet {}",
                self.state.explorers[&explorer_id].current_planet
            ));
        }

        let neighbors = self.state.galaxy.get_planet_neighbours(current_planet_id);
        self.state.explorers_communication_center.send_to(
            explorer_id,
            OrchestratorToExplorer::NeighborsResponse { neighbors },
        )
    }

    fn handle_travel_request(
        &mut self,
        explorer_id: ID,
        current_planet_id: ID,
        dst_planet_id: ID,
    ) -> Result<(), String> {
        if current_planet_id != self.state.explorers[&explorer_id].current_planet {
            return Err(format!(
                "Explorer {explorer_id} requested travel from planet {current_planet_id}, but is currently on planet {}",
                self.state.explorers[&explorer_id].current_planet
            ));
        }

        // Communicate invalid travel if planets are not connected
        if !self
            .state
            .galaxy
            .are_planets_connected(current_planet_id, dst_planet_id)
        {
            return self.notify_explorer_invalid_movement(explorer_id, current_planet_id);
        }

        let new_sender = self.state.explorers[&explorer_id].tx_planet.clone();
        self.state.planets_communication_center.notify_planet_incoming_explorer(explorer_id, dst_planet_id, new_sender)?;
        self.state.planets_communication_center.notify_planet_explorer_left(explorer_id, current_planet_id)?;
        let new_sender = self.state.planets[&dst_planet_id].tx_explorer.clone();
        self.state.explorers_communication_center.notify_explorer_successful_movement(explorer_id, dst_planet_id, new_sender)?;

        // Update internal state
        self.state
            .explorers
            .get_mut(&explorer_id)
            .unwrap()
            .current_planet = dst_planet_id;
        self.state
            .gui_events_buffer
            .explorer_moved(explorer_id, dst_planet_id);

        Ok(())
    }

    fn notify_explorer_invalid_movement(
        &mut self,
        explorer_id: ID,
        current_planet_id: ID,
    ) -> Result<(), String> {
        let moved_planet_id = self
            .state
            .explorers_communication_center
            .req_ack(
                explorer_id,
                OrchestratorToExplorer::MoveToPlanet {
                    sender_to_new_planet: None,
                    planet_id: current_planet_id,
                },
                ExplorerToOrchestratorKind::MovedToPlanetResult,
            )?
            .into_moved_to_planet_result()
            .unwrap()
            .1; // Unwrap is safe due to expected kind

        if moved_planet_id != current_planet_id {
            return Err(format!(
                "Explorer {explorer_id} moved to planet {moved_planet_id}, but was expected to stay on planet {current_planet_id}"
            ));
        }
        Ok(())
    }
}

impl OrchestratorUpdateStrategy for AutoUpdateStrategy<'_> {
    fn update(&mut self) -> Result<(), String> {
        self.execute_cycle()
    }

    fn process_commands(&mut self) -> Result<(), String> {
        log::warn!("AutoUpdateStrategy does not process commands");
        Ok(())
    }
}

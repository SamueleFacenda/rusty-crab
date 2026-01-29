use common_game::components::asteroid::Asteroid;
use common_game::components::sunray::Sunray;
use common_game::protocols::orchestrator_explorer::OrchestratorToExplorer;
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::utils::ID;

use crate::app::AppConfig;
use crate::orchestrator::OrchestratorState;
use crate::orchestrator::probability::ProbabilityCalculator;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;

pub(crate) struct AutoUpdateStrategy {
}

impl AutoUpdateStrategy {
    fn execute_cycle(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        self.send_sunrays(state)?;
        self.send_asteroids(state)?;
        self.send_bag_content_requests(state)?;

        Ok(())
    }

    fn send_asteroids(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        for planet_id in state.galaxy.get_planets() {
            if rand::random::<f32>() < ProbabilityCalculator::get_asteroid_probability(state.time) {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Asteroid(Asteroid::default()), state)? {
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: Some(_) } => {}, // planet defended itself
                    PlanetToOrchestrator::AsteroidAck { planet_id, rocket: None } => { // handle destroyed planet
                        state.handle_planet_destroyed(planet_id);
                    },
                    _ => return Err(format!("Unexpected response from planet {planet_id} to asteroid (invalid state)")),
                }
            }
        }
        Ok(())
    }

    fn send_sunrays(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        for planet_id in state.galaxy.get_planets() {
            if rand::random::<f32>() < ProbabilityCalculator::get_sunray_probability(state.time) {
                match self.planet_syn_ack(planet_id, OrchestratorToPlanet::Sunray(Sunray::default()), state)? {
                    PlanetToOrchestrator::SunrayAck{planet_id} => {}, // planet handled sunray
                    _ => return Err(format!("Unexpected response from planet {planet_id} to sunray (invalid state)")),
                }
            }
        }
        Ok(())
    }

    fn planet_syn_ack(&self, planet_id: ID, msg: OrchestratorToPlanet, state: &mut OrchestratorState) -> Result<PlanetToOrchestrator, String> {
        state.planets[&planet_id]
            .tx
            .send(msg, planet_id)
            .map_err(|e| e.to_string())?;
        state.planets_rx
            .recv_timeout(AppConfig::get().max_wait_time_ms, planet_id)
            .map(|r| if r.planet_id() == planet_id {
                Ok(r)
            } else {
                Err(format!("The wrong planet responded! Expected {planet_id}, got {}", r.planet_id()))
            } )
            .map_err(|e| e.to_string())? // The ? is because it's a nested Result
    }

    fn send_bag_content_requests(&self, state: &mut OrchestratorState) -> Result<(), String> {
        for (id, explorer_handle) in &state.explorers {
            explorer_handle
                .tx
                .send(OrchestratorToExplorer::BagContentRequest, *id)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

impl OrchestratorUpdateStrategy for AutoUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        self.execute_cycle(state)
    }
}
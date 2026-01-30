mod auto_update_strategy;
mod manual_update_strategy;

use crate::orchestrator::{OrchestratorMode, OrchestratorState};

pub trait OrchestratorUpdateStrategy {
    fn update(&mut self) -> Result<(), String>;
    fn process_commands(&mut self) -> Result<(), String>;
}

pub struct OrchestratorUpdateFactory;

impl OrchestratorUpdateFactory {
    pub fn get_strategy(mode: OrchestratorMode, state: &mut OrchestratorState) -> Box<dyn OrchestratorUpdateStrategy + '_> {
        match mode {
            OrchestratorMode::Auto => Box::new(auto_update_strategy::AutoUpdateStrategy::new(state)),
            OrchestratorMode::Manual => Box::new(manual_update_strategy::ManualUpdateStrategy::new(state)),
        }
    }
}

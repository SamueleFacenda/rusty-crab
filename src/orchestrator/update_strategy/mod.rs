mod auto_update_strategy;
mod manual_update_strategy;

use crate::orchestrator::{OrchestratorMode, OrchestratorState};

pub trait OrchestratorUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String>;
}

pub struct OrchestratorUpdateFactory;

impl OrchestratorUpdateFactory {
    pub fn get_strategy(mode: OrchestratorMode) -> Box<dyn OrchestratorUpdateStrategy> {
        match mode {
            OrchestratorMode::Auto => Box::new(auto_update_strategy::AutoUpdateStrategy::new()),
            OrchestratorMode::Manual => Box::new(manual_update_strategy::ManualUpdateStrategy::new()),
        }
    }
}

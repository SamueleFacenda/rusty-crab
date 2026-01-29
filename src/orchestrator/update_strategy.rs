use crate::orchestrator::auto_update_strategy::AutoUpdateStrategy;
use crate::orchestrator::manual_update_strategy::ManualUpdateStrategy;
use crate::orchestrator::core::OrchestratorMode;
use crate::orchestrator::OrchestratorState;

pub trait OrchestratorUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String>;
}

pub struct OrchestratorUpdateFactory;

impl OrchestratorUpdateFactory {
    pub fn get_strategy(mode: OrchestratorMode) -> Box<dyn OrchestratorUpdateStrategy> {
        match mode {
            OrchestratorMode::Auto => Box::new(AutoUpdateStrategy::new()),
            OrchestratorMode::Manual => Box::new(ManualUpdateStrategy {}),
        }
    }
}
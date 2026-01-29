use crate::orchestrator::OrchestratorState;
use crate::orchestrator::update_strategy::OrchestratorUpdateStrategy;

pub(crate) struct ManualUpdateStrategy;

impl OrchestratorUpdateStrategy for ManualUpdateStrategy {
    fn update(&mut self, state: &mut OrchestratorState) -> Result<(), String> {
        // In manual mode, we do not perform any automatic updates.
        Ok(())
    }
}

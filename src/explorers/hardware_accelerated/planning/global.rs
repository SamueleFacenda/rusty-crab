use common_game::components::resource::{ComplexResourceType, ResourceType};

use super::super::ExplorerState;

const FINAL_RESOURCES: [ComplexResourceType; 2] = [ComplexResourceType::AIPartner, ComplexResourceType::Dolphin];

#[derive(Debug)]
pub(crate) struct GlobalTask {
    pub resource: ComplexResourceType
}

impl GlobalTask {
    pub fn new(resource: ComplexResourceType) -> Self { GlobalTask { resource } }
}

pub(crate) struct GlobalPlanner;

/// Tries to have equal count of each final resource in the bag
impl GlobalPlanner {
    pub fn plan_next_task(state: &ExplorerState) -> GlobalTask {
        let min_res = FINAL_RESOURCES
            .iter()
            .map(|res_type| {
                let count = state.bag.res.get(&ResourceType::Complex(*res_type)).map(|v| v.len()).unwrap_or_default();
                (res_type, count)
            })
            .min_by(|a, b| a.1.cmp(&b.1))
            .unwrap()
            .0;
        GlobalTask::new(*min_res)
    }
}

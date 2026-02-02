use common_game::components::resource::{BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

use crate::gui::types::OrchestratorEvent;

pub struct GuiEventBuffer {
    buffer: Vec<OrchestratorEvent>
}

impl GuiEventBuffer {
    pub fn new() -> Self { GuiEventBuffer { buffer: Vec::new() } }

    pub fn sunray_sent(&mut self, planet_id: ID) { self.buffer.push(OrchestratorEvent::SunraySent { planet_id }); }

    pub fn sunray_received(&mut self, planet_id: ID) {
        self.buffer.push(OrchestratorEvent::SunrayReceived { planet_id });
    }

    pub fn planet_destroyed(&mut self, planet_id: ID) {
        self.buffer.push(OrchestratorEvent::PlanetDestroyed { planet_id });
    }

    pub fn asteroid_sent(&mut self, planet_id: ID) { self.buffer.push(OrchestratorEvent::AsteroidSent { planet_id }); }

    pub fn explorer_moved(&mut self, explorer_id: ID, destination: ID) {
        self.buffer.push(OrchestratorEvent::ExplorerMoved { explorer_id, destination });
    }

    pub fn basic_resource_generated(&mut self, explorer_id: ID, resource: BasicResourceType) {
        self.buffer.push(OrchestratorEvent::BasicResourceGenerated { explorer_id, resource });
    }

    pub fn complex_resource_generated(&mut self, explorer_id: ID, resource: ComplexResourceType) {
        self.buffer.push(OrchestratorEvent::ComplexResourceGenerated { explorer_id, resource });
    }

    pub fn drain_events(&mut self) -> Vec<OrchestratorEvent> { std::mem::take(&mut self.buffer) }
    pub fn has_events(&self) -> bool { !self.buffer.is_empty() }
}

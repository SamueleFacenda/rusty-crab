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

    pub fn explorer_moved(&mut self, origin: ID, destination: ID) {
        self.buffer.push(OrchestratorEvent::ExplorerMoved { origin, destination });
    }

    pub fn drain_events(&mut self) -> Vec<OrchestratorEvent> { std::mem::take(&mut self.buffer) }
}

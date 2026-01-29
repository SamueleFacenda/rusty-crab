use std::collections::{HashMap, VecDeque};

use common_game::utils::ID;

use crate::app::AppConfig;
use crate::orchestrator::logging_channel::{
    ActorMarker, ExplorerMarker, LoggingReceiver, PlanetMarker,
};

/// This wrapper around a channel receiver divides the stream per sender (planet and explorer IDs).
pub(crate) struct ChannelDemultiplexer<A: ActorMarker> {
    receiver: LoggingReceiver<A>,
    buffers: HashMap<ID, VecDeque<A::RecvMsg>>,
}

impl<A: ActorMarker> ChannelDemultiplexer<A> {
    pub fn new(receiver: LoggingReceiver<A>) -> Self {
        Self {
            receiver,
            buffers: HashMap::new(),
        }
    }

    pub fn recv_from(&mut self, id: ID) -> Result<A::RecvMsg, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);
        // Check if we have buffered messages for this ID
        if let Some(buffer) = self.buffers.get_mut(&id)
            && let Some(msg) = buffer.pop_front()
        {
            return Ok(msg);
        }

        // Keep receiving until we find a message from the desired ID or timeout
        let start_time = std::time::Instant::now();
        while start_time.elapsed() < timeout {
            let msg = self
                .receiver
                .recv_timeout(timeout)
                .map_err(|e| e.to_string())?;
            let msg_id = A::get_id(&msg);

            if msg_id == id {
                return Ok(msg);
            }

            // Buffer the message for later
            self.buffers.entry(msg_id).or_default().push_back(msg);
        }
        Err(format!("Timeout waiting for message from ID {id}"))
    }
}

// Convenience type aliases
pub type PlanetChannelDemultiplexer = ChannelDemultiplexer<PlanetMarker>;
pub type ExplorerChannelDemultiplexer = ChannelDemultiplexer<ExplorerMarker>;

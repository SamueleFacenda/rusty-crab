use std::collections::{HashMap, VecDeque};

use common_game::utils::ID;

use super::logging_channel::{ActorMarker, ExplorerMarker, LoggingReceiver, PlanetMarker};
use crate::app::AppConfig;

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
                .map_err(|e| format!("Error waiting for message from ID {id}: {e}"))?;
            let msg_id = A::get_id(&msg);

            if msg_id == id {
                return Ok(msg);
            }

            // Buffer the message for later
            self.buffers.entry(msg_id).or_default().push_back(msg);
        }
        Err(format!("Timeout waiting for message from ID {id}"))
    }

    /// Doesn't require mut, just receives the next available message from any sender.
    pub fn recv_any(&self) -> Result<A::RecvMsg, String> {
        let timeout = std::time::Duration::from_millis(AppConfig::get().max_wait_time_ms);

        self.receiver
            .recv_timeout(timeout)
            .map_err(|e| format!("Error waiting for message: {e}"))
    }
}

// Convenience type aliases
pub type PlanetChannelDemultiplexer = ChannelDemultiplexer<PlanetMarker>;
pub type ExplorerChannelDemultiplexer = ChannelDemultiplexer<ExplorerMarker>;


#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use crate::explorers::BagContent;
    use common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator;

    fn msg(id: ID) -> ExplorerToOrchestrator<BagContent> {
        ExplorerToOrchestrator::StartExplorerAIResult { explorer_id: id }
    }

    fn make_mux() -> (
        crossbeam_channel::Sender<ExplorerToOrchestrator<BagContent>>,
        ExplorerChannelDemultiplexer,
    ) {
        let (tx, rx) = unbounded();
        let logging_rx = LoggingReceiver::<ExplorerMarker>::new(rx);
        let demux = ExplorerChannelDemultiplexer::new(logging_rx);
        (tx, demux)
    }

    #[test]
    fn receive_with_id() {
        let (tx, mut mux) = make_mux();
        tx.send(msg(1)).unwrap();
        let received = mux.recv_from(1).unwrap();
        // Unwrap panics if there is no message
    }

    #[test]
    fn receive_with_more_ids() {
        // This tests also tests message queueing
        let (tx, mut mux) = make_mux();
        tx.send(msg(2)).unwrap();
        tx.send(msg(2)).unwrap();
        tx.send(msg(1)).unwrap();
        tx.send(msg(2)).unwrap();

        let received = mux.recv_from(1).unwrap();
        assert_eq!(received.explorer_id(), 1);
        let received = mux.recv_from(2).unwrap();
        assert_eq!(received.explorer_id(), 2);
        let received = mux.recv_from(2).unwrap();
        assert_eq!(received.explorer_id(), 2);
        let received = mux.recv_from(2).unwrap();
        assert_eq!(received.explorer_id(), 2);
    }

    #[test]
    fn receive_missing_id() {
        let (tx, mut mux) = make_mux();

        let result = mux.recv_from(67);
        assert!(result.is_err());
    }

}
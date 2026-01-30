//! Woo AOP logging channel for orchestrator would be awesome.
//! These are wrappers around crossbeam channels that log send and receive events
//! using the common logging infrastructure.
use std::marker::PhantomData;

use common_game::logging::ActorType::{Explorer, Orchestrator, Planet};
use common_game::logging::Channel::Debug;
use common_game::logging::EventType::{
    MessageExplorerToOrchestrator, MessageOrchestratorToExplorer, MessageOrchestratorToPlanet,
    MessagePlanetToOrchestrator,
};
use common_game::logging::{EventType, LogEvent, Participant, Payload};
use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator, OrchestratorToExplorer,
};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::BagContent;

const ORCHESTRATOR_PARTICIPANT: Option<Participant> = Some(Participant {
    actor_type: Orchestrator,
    id: 0,
});

// Marker types for different actors
pub struct ExplorerMarker;
pub struct PlanetMarker;

pub trait ActorMarker {
    type SendMsg: std::fmt::Debug;
    type RecvMsg: std::fmt::Debug;
    fn event_type_send() -> EventType;
    fn event_type_recv() -> EventType;
    fn actor_type() -> common_game::logging::ActorType;
    fn get_id(msg: &Self::RecvMsg) -> ID;
}

impl ActorMarker for ExplorerMarker {
    type SendMsg = OrchestratorToExplorer;
    type RecvMsg = ExplorerToOrchestrator<BagContent>;
    fn event_type_send() -> EventType {
        MessageOrchestratorToExplorer
    }
    fn event_type_recv() -> EventType {
        MessageExplorerToOrchestrator
    }
    fn actor_type() -> common_game::logging::ActorType {
        Explorer
    }
    fn get_id(msg: &Self::RecvMsg) -> ID {
        msg.explorer_id()
    }
}

impl ActorMarker for PlanetMarker {
    type SendMsg = OrchestratorToPlanet;
    type RecvMsg = PlanetToOrchestrator;
    fn event_type_send() -> EventType {
        MessageOrchestratorToPlanet
    }
    fn event_type_recv() -> EventType {
        MessagePlanetToOrchestrator
    }
    fn actor_type() -> common_game::logging::ActorType {
        Planet
    }
    fn get_id(msg: &Self::RecvMsg) -> ID {
        msg.planet_id()
    }
}

pub struct LoggingSender<A: ActorMarker> {
    sender: Sender<A::SendMsg>,
    _marker: PhantomData<A>,
}

pub struct LoggingReceiver<A: ActorMarker> {
    receiver: Receiver<A::RecvMsg>,
    _marker: PhantomData<A>,
}

impl<A: ActorMarker> LoggingSender<A> {
    pub fn new(sender: Sender<A::SendMsg>) -> Self {
        Self {
            sender,
            _marker: PhantomData,
        }
    }

    pub fn send(
        &self,
        msg: A::SendMsg,
        id: ID,
    ) -> Result<(), crossbeam_channel::SendError<A::SendMsg>> {
        LogEvent::new(
            ORCHESTRATOR_PARTICIPANT,
            Some(Participant {
                actor_type: A::actor_type(),
                id,
            }),
            A::event_type_send(),
            Debug,
            Payload::from([("msg".to_string(), format!("{msg:?}"))]),
        )
        .emit();
        self.sender.send(msg)
    }
}

impl<A: ActorMarker> LoggingReceiver<A> {
    pub fn new(receiver: Receiver<A::RecvMsg>) -> Self {
        Self {
            receiver,
            _marker: PhantomData,
        }
    }

    #[allow(dead_code)] // kept for completeness
    pub fn recv(&self) -> Result<A::RecvMsg, crossbeam_channel::RecvError> {
        self.receiver
            .recv()
            .inspect(|msg| Self::log(msg, A::get_id(msg))) // Log only successful receives
    }

    #[allow(dead_code)] // kept for completeness
    pub fn try_recv(&self) -> Result<A::RecvMsg, crossbeam_channel::TryRecvError> {
        self.receiver
            .try_recv()
            .inspect(|msg| Self::log(msg, A::get_id(msg))) // Log only successful receives
    }

    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<A::RecvMsg, crossbeam_channel::RecvTimeoutError> {
        self.receiver
            .recv_timeout(timeout)
            .inspect(|msg| Self::log(msg, A::get_id(msg))) // Log only successful receives
    }

    fn log(msg: &A::RecvMsg, id: ID) {
        LogEvent::new(
            Some(Participant {
                actor_type: A::actor_type(),
                id,
            }),
            ORCHESTRATOR_PARTICIPANT,
            A::event_type_recv(),
            Debug,
            Payload::from([("msg".to_string(), format!("{msg:?}"))]),
        )
        .emit();
    }
}

// Type aliases for convenience
pub type ExplorerLoggingSender = LoggingSender<ExplorerMarker>;
pub type PlanetLoggingSender = LoggingSender<PlanetMarker>;
pub type ExplorerLoggingReceiver = LoggingReceiver<ExplorerMarker>;
pub type PlanetLoggingReceiver = LoggingReceiver<PlanetMarker>;


#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use super::*;
    use crossbeam_channel::unbounded;
    use std::time::Duration;

    #[test]
    fn send_message() {
        let (tx, rx) = unbounded();
        let logging_sender = ExplorerLoggingSender::new(tx);

        let msg = OrchestratorToExplorer::StartExplorerAI;
        let id = 1;

        logging_sender.send(msg, id).unwrap();
        let received = rx.recv().unwrap();//Unwrap tests if the message is there
        match received {
            OrchestratorToExplorer::StartExplorerAI => {},
            other => unreachable!()
        }
    }

    #[test]
    fn recv_timeout_from_empty() {
        let (tx, rx) = unbounded();
        let logging_receiver = ExplorerLoggingReceiver::new(rx);
        assert!(logging_receiver.recv_timeout(Duration::from_millis(2000)).is_err());
    }

    #[test]
    fn recv_timeout_message() {
        let (tx, rx) = unbounded();
        let logging_receiver = ExplorerLoggingReceiver::new(rx);

        let id = 1;
        let msg = ExplorerToOrchestrator::StartExplorerAIResult {
            explorer_id: id,
        };
        tx.send(msg).unwrap();
        let received = logging_receiver.recv_timeout(
            Duration::from_millis(5000)
        ).unwrap();
        match received {
            ExplorerToOrchestrator::StartExplorerAIResult {..} => {},
            other => unreachable!()
        }
    }

    #[test]
    fn recv_message() {
        let (tx, rx) = unbounded();
        let logging_receiver = ExplorerLoggingReceiver::new(rx);

        let id = 1;
        let msg = ExplorerToOrchestrator::StartExplorerAIResult {
            explorer_id: id,
        };
        tx.send(msg).unwrap();
        let received = logging_receiver.recv().unwrap();
        match received {
            ExplorerToOrchestrator::StartExplorerAIResult {..} => {},
            other => unreachable!()
        }
    }
}

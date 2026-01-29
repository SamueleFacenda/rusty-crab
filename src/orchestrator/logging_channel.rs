use common_game::utils::ID;
use common_game::logging::ActorType::{Explorer, Orchestrator, Planet};
use common_game::logging::Channel::{Debug, Info};
use common_game::logging::EventType::{MessagePlanetToOrchestrator, MessageExplorerToOrchestrator, MessageOrchestratorToExplorer, MessageOrchestratorToPlanet};
use common_game::logging::{EventType, LogEvent, Participant, Payload};

use crossbeam_channel::{Sender, Receiver};
use std::marker::PhantomData;
use std::time::Duration;
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::orchestrator_planet::{OrchestratorToPlanet, PlanetToOrchestrator};
use crate::orchestrator::BagContent;

/// Woo AOP logging channel for orchestrator would be awesome
/// These are wrappers around crossbeam channels that log send and receive events
/// using the common logging infrastructure.

const ORCHESTRATOR_PARTICIPANT: Option<Participant> = Some(Participant {
    actor_type: Orchestrator,
    id: 0,
});


// Marker types for different actors
pub struct ExplorerMarker;
pub struct PlanetMarker;

pub trait ActorMarker {
    fn event_type_send() -> EventType;
    fn event_type_recv() -> EventType;
    fn actor_type() -> common_game::logging::ActorType;
}

impl ActorMarker for ExplorerMarker {
    fn event_type_send() -> EventType { MessageOrchestratorToExplorer }
    fn event_type_recv() -> EventType { MessageExplorerToOrchestrator }
    fn actor_type() -> common_game::logging::ActorType { Explorer }
}

impl ActorMarker for PlanetMarker {
    fn event_type_send() -> EventType { MessageOrchestratorToPlanet }
    fn event_type_recv() -> EventType { MessagePlanetToOrchestrator }
    fn actor_type() -> common_game::logging::ActorType { Planet }
}

pub struct LoggingSender<T, A: ActorMarker> {
    sender: Sender<T>,
    _marker: PhantomData<A>,
}

pub struct LoggingReceiver<T, A: ActorMarker> {
    receiver: Receiver<T>,
    _marker: PhantomData<A>,
}

impl<T: std::fmt::Debug, A: ActorMarker> LoggingSender<T, A> {
    pub fn new(sender: Sender<T>) -> Self {
        Self { sender, _marker: PhantomData }
    }

    pub fn send(&self, msg: T, id: ID) -> Result<(), crossbeam_channel::SendError<T>> {
        LogEvent::new(ORCHESTRATOR_PARTICIPANT,
                      Some(Participant { actor_type: A::actor_type(), id }),
                      A::event_type_send(),
                      Debug,
                      Payload::from([("msg".to_string(), format!("{msg:?}"))]),
        ).emit();
        self.sender.send(msg)
    }
}

impl<T: std::fmt::Debug, A: ActorMarker> LoggingReceiver<T, A> {
    pub fn new(receiver: Receiver<T>) -> Self {
        Self { receiver, _marker: PhantomData }
    }

    pub fn recv(&self, id: ID) -> Result<T, crossbeam_channel::RecvError> {
        self.receiver.recv().map(|msg| {
            // Log only successful receives
            self.log(&msg, id);
            msg
        })
    }

    pub fn try_recv(&self, id: ID) -> Result<T, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv().map(|msg| {
            // Log only successful receives
            self.log(&msg, id);
            msg
        })
    }

    pub fn recv_timeout(&self, timeout_ms: u64, id: ID) -> Result<T, crossbeam_channel::RecvTimeoutError> {
        self.receiver.recv_timeout(Duration::from_millis(timeout_ms)).map(|msg| {
            // Log only successful receives
            self.log(&msg, id);
            msg
        })
    }

    fn log(&self, msg: &T, id: ID) {
        LogEvent::new(
            Some(Participant { actor_type: A::actor_type(), id }),
            ORCHESTRATOR_PARTICIPANT,
            A::event_type_recv(),
            Debug,
            Payload::from([("msg".to_string(), format!("{msg:?}"))]),
        ).emit();
    }
}

// Type aliases for convenience
pub type ExplorerLoggingSender = LoggingSender<OrchestratorToExplorer, ExplorerMarker>;
pub type PlanetLoggingSender = LoggingSender<OrchestratorToPlanet, PlanetMarker>;
pub type ExplorerLoggingReceiver = LoggingReceiver<ExplorerToOrchestrator<BagContent>, ExplorerMarker>;
pub type PlanetLoggingReceiver = LoggingReceiver<PlanetToOrchestrator, PlanetMarker>;

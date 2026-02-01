//! Similar to logging channels for orchestrator, but the other way around.
//! Important difference: the ID of the explorer is dynamic and so is the other actor's ID.
//! They cannot be extrapolated from the message itself.
use std::marker::PhantomData;

use common_game::logging::ActorType::{Explorer, Orchestrator, Planet};
use common_game::logging::Channel::Debug;
use common_game::logging::EventType::{MessageExplorerToOrchestrator, MessageExplorerToPlanet,
                                      MessageOrchestratorToExplorer, MessagePlanetToExplorer};
use common_game::logging::{EventType, LogEvent, Participant, Payload};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use crate::explorers::BagContent;

// Marker types for different actors
pub struct OrchestratorMarker;
pub struct PlanetMarker;

pub trait ActorMarker {
    type SendMsg: std::fmt::Debug;
    type RecvMsg: std::fmt::Debug;
    fn event_type_send() -> EventType;
    fn event_type_recv() -> EventType;
    fn actor_type() -> common_game::logging::ActorType;
}

impl ActorMarker for OrchestratorMarker {
    type SendMsg = ExplorerToOrchestrator<BagContent>;
    type RecvMsg = OrchestratorToExplorer;
    fn event_type_send() -> EventType { MessageExplorerToOrchestrator }
    fn event_type_recv() -> EventType { MessageOrchestratorToExplorer }
    fn actor_type() -> common_game::logging::ActorType { Orchestrator }
}

impl ActorMarker for PlanetMarker {
    type SendMsg = ExplorerToPlanet;
    type RecvMsg = PlanetToExplorer;
    fn event_type_send() -> EventType { MessageExplorerToPlanet }
    fn event_type_recv() -> EventType { MessagePlanetToExplorer }
    fn actor_type() -> common_game::logging::ActorType { Planet }
}

pub struct LoggingSender<A: ActorMarker> {
    sender: Sender<A::SendMsg>,
    explorer_id: ID,
    other_id: ID,
    _marker: PhantomData<A>
}

pub struct LoggingReceiver<A: ActorMarker> {
    receiver: Receiver<A::RecvMsg>,
    explorer_id: ID,
    other_id: ID,
    _marker: PhantomData<A>
}

impl<A: ActorMarker> LoggingSender<A> {
    pub fn new(sender: Sender<A::SendMsg>, explorer_id: ID, other_id: ID) -> Self {
        Self { sender, explorer_id, other_id, _marker: PhantomData }
    }

    pub fn send(&self, msg: A::SendMsg) -> Result<(), String> {
        LogEvent::new(
            Some(Participant { actor_type: Explorer, id: self.explorer_id }),
            Some(Participant { actor_type: A::actor_type(), id: self.other_id }),
            A::event_type_send(),
            Debug,
            Payload::from([("msg".to_string(), format!("{msg:?}"))])
        )
        .emit();
        self.sender.send(msg).map_err(|e| e.to_string())
    }
}

impl<A: ActorMarker> LoggingReceiver<A> {
    pub fn new(receiver: Receiver<A::RecvMsg>, explorer_id: ID, other_id: ID) -> Self {
        Self { receiver, explorer_id, other_id, _marker: PhantomData }
    }

    pub fn set_other_id(&mut self, other_id: ID) { self.other_id = other_id; }

    #[allow(dead_code)] // kept for completeness
    pub fn recv(&self) -> Result<A::RecvMsg, crossbeam_channel::RecvError> {
        self.receiver.recv().inspect(|msg| self.log(msg)) // Log only successful receives
    }

    #[allow(dead_code)] // kept for completeness
    pub fn try_recv(&self) -> Result<A::RecvMsg, crossbeam_channel::TryRecvError> {
        self.receiver.try_recv().inspect(|msg| self.log(msg)) // Log only successful receives
    }

    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration
    ) -> Result<A::RecvMsg, crossbeam_channel::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout).inspect(|msg| self.log(msg)) // Log only successful receives
    }

    fn log(&self, msg: &A::RecvMsg) {
        LogEvent::new(
            Some(Participant { actor_type: A::actor_type(), id: self.other_id }),
            Some(Participant { actor_type: Explorer, id: self.explorer_id }),
            A::event_type_recv(),
            Debug,
            Payload::from([("msg".to_string(), format!("{msg:?}"))])
        )
        .emit();
    }
}

// Type aliases for convenience
pub type OrchestratorLoggingSender = LoggingSender<OrchestratorMarker>;
pub type PlanetLoggingSender = LoggingSender<PlanetMarker>;
pub type OrchestratorLoggingReceiver = LoggingReceiver<OrchestratorMarker>;
pub type PlanetLoggingReceiver = LoggingReceiver<PlanetMarker>;

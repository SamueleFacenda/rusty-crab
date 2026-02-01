use common_game::utils::ID;
use std::collections::HashMap;
use common_game::protocols::planet_explorer::ExplorerToPlanet;
use crossbeam_channel::Sender;
use crate::orchestrator::communication::channel_demultiplexer::ChannelDemultiplexer;
use crate::orchestrator::communication::logging_channel::{ActorMarker, ExplorerMarker, LoggingSender, PlanetMarker};

/// Like a control tower, this struct provides utilities and logic handling for communication
pub(crate) struct CommunicationCenter<A: ActorMarker> {
    pub tx: HashMap<ID, LoggingSender<A>>,
    pub rx: ChannelDemultiplexer<A>,
}

impl<A: ActorMarker> CommunicationCenter<A> {
    pub fn new(
        tx: HashMap<ID, LoggingSender<A>>,
        rx: ChannelDemultiplexer<A>,
    ) -> Self {
        CommunicationCenter {
            tx,
            rx,
        }
    }
    
    pub fn remove(&mut self, id: ID) {
        self.tx.remove(&id);
    }

    pub fn send_to(&self, id: ID, msg: A::SendMsg) -> Result<(), String> {
        self.tx[&id]
            .send(msg, id)
            .map_err(|e| e.to_string())
    }

    pub fn req_ack(
        &mut self,
        id: ID,
        msg: A::SendMsg,
        expected: A::RecvMsgKind,
    ) -> Result<A::RecvMsg, String>{
        self.send_to(id, msg)?;
        self.recv_from(id).map(|res| {
            if A::RecvMsgKind::from(&res) == expected {
                Ok(res)
            } else {
                Err(format!(
                    "Expected {} {id} to respond with {expected:?}, but got {res:?}", A::get_name()
                ))
            }
        })? // Flatten the Result<Result<...>>
    }

    /// Same asreq_ack but doesn't require &mut self. Doesn't buffer messages.
    /// May lead to lost messages if another actor sends a message while waiting for the response.
    pub fn riskier_req_ack(
        &self,
        id: ID,
        msg: A::SendMsg,
        expected: A::RecvMsgKind,
    ) -> Result<A::RecvMsg, String> {
        self.send_to(id, msg)?;
        match self.rx.recv_any() {
            Ok(res) => {
                if A::get_id(&res) != id {
                    return Err(format!(
                        "Expected response from Þ{} {id}, but got message from {} {}: {res:?}",
                        A::get_name(),
                        A::get_name(),
                        A::get_id(&res)
                    ));
                }
                if A::RecvMsgKind::from(&res) != expected {
                    return Err(format!(
                        "Expected {} {id} to respond with {expected:?}, but got {res:?}",
                        A::get_name()
                    ));
                }
                Ok(res)
            }
            Err(e) => Err(format!(
                "Error receiving response from {} {id}: {e}", A::get_name()
            )),
        }
    }

    pub fn recv_from(
        &mut self,
        id: ID,
    ) -> Result<A::RecvMsg, String> {
        self.rx.recv_from(id)
    }
}



pub(crate) type PlanetCommunicationCenter = CommunicationCenter<PlanetMarker>;
pub(crate) type ExplorerCommunicationCenter = CommunicationCenter<ExplorerMarker>;

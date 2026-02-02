//! Communication module for orchestrator handling inter-component messaging.
//! These structs wrap crossbeam channels with logging and demultiplexing capabilities.
//! The communication center provides a simplified API for sending and receiving messages (e.g.
//! request-acknowledge patterns) between the orchestrator, explorers, and planets.
mod channel_demultiplexer;
mod communication_center;
mod explorers_communication_center;
mod logging_channel;
mod planets_communication_center;

pub(super) use channel_demultiplexer::{ExplorerChannelDemultiplexer, PlanetChannelDemultiplexer};
pub(super) use communication_center::{ExplorerCommunicationCenter, PlanetCommunicationCenter};
pub(super) use logging_channel::{ExplorerLoggingReceiver, ExplorerLoggingSender, PlanetLoggingReceiver,
                                 PlanetLoggingSender};

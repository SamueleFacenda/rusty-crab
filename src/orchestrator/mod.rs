//! Module that contains the orchestrator
mod communication;
mod core;
mod galaxy;
mod init;
mod probability;
mod state;
mod update_strategy;

pub(crate) use core::{Orchestrator, OrchestratorMode};
pub(crate) use init::{GalaxyBuilder, PLANET_ORDER, PlanetFactory, PlanetType};
pub(crate) use state::{ExplorerHandle, OrchestratorManualAction, OrchestratorState, PlanetHandle};

use communication::{ExplorerChannelDemultiplexer, ExplorerLoggingReceiver, ExplorerLoggingSender,
                               PlanetChannelDemultiplexer, PlanetLoggingReceiver, PlanetLoggingSender};
use probability::ProbabilityCalculator;
use galaxy::Galaxy;
use update_strategy::OrchestratorUpdateFactory;

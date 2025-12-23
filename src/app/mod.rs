//! This module contains app and lifecycle related code. Like the config management, and the logging.

mod logging;
mod config;

pub(crate) use config::AppConfig;
pub(crate) use logging::setup_logger;

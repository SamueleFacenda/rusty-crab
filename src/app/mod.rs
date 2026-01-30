//! This module contains app and lifecycle related code. Like the config management, and the logging.

mod config;
mod logging;

pub(crate) use config::AppConfig;
pub(crate) use logging::setup_logger;

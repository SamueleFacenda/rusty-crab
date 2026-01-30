//! Module containing the orchestrator initialization routines.
//! This includes galaxy building and planet factory logic, with all the
//! channels setup and sharing.
//!
mod galaxy_builder;
mod planet_factory;

pub(crate) use galaxy_builder::GalaxyBuilder;
pub(crate) use planet_factory::{PlanetFactory, PlanetType};

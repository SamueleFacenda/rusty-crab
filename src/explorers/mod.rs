//! Definition of explorers traits and export of explorer builders.
//! All the different explorers lives in a submodule.

mod example;
mod explorer;

pub(crate) use explorer::{BagContent, Explorer, ExplorerBuilder};

pub(crate) type ExampleExplorerBuilder = explorer::ExplorerBuilderImpl<example::ExampleExplorer>;

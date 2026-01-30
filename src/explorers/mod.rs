//! Definition of explorers traits and export of explorer builders.
//! All the different explorers lives in a submodule.

mod example;
mod explorer;

pub(crate) use explorer::{BagContent, Explorer, ExplorerBuilder};

pub(crate) type ExampleExplorerBuilder = explorer::ExplorerBuilderImpl<example::ExampleExplorer>;

pub(crate) struct ExplorerFactory;

impl ExplorerFactory {
    pub fn make_from_name(type_name: &String) -> Box<dyn ExplorerBuilder> {
        match type_name.to_ascii_lowercase().as_str() {
            "example" => Box::new(ExampleExplorerBuilder::new()),
            _ => {
                log::warn!("Explorer type '{type_name}' not recognized. Defaulting to 'example' explorer.");
                Box::new(ExampleExplorerBuilder::new())
            },
        }
    }
}
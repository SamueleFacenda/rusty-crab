//! Definition of explorers traits and export of explorer builders.
//! All the different explorers lives in a submodule.

mod example;
mod explorer;
mod samufaz;

pub(crate) use explorer::{BagContent, Explorer, ExplorerBuilder};

pub(crate) type ExampleExplorerBuilder = explorer::ExplorerBuilderImpl<example::ExampleExplorer>;
pub(crate) type SamuFazExplorerBuilder =
    explorer::ExplorerBuilderImpl<samufaz::SamuFazExplorer>;

pub(crate) struct ExplorerFactory;

impl ExplorerFactory {
    pub fn make_from_name(type_name: &String) -> Box<dyn ExplorerBuilder> {
        #[allow(clippy::single_match_else)] // more explorers are added in personal branches
        match type_name.to_ascii_lowercase().as_str() {
            "example" => Box::new(ExampleExplorerBuilder::new()),
            "samufaz" => Box::new(SamuFazExplorerBuilder::new()),
            _ => {
                log::warn!("Explorer type '{type_name}' not recognized. Defaulting to 'example' explorer.");
                Box::new(ExampleExplorerBuilder::new())
            }
        }
    }
}

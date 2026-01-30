mod app;
mod orchestrator;
mod explorers;

use orchestrator::{Orchestrator, OrchestratorMode};
use crate::explorers::ExplorerBuilder;

fn init() {
    app::AppConfig::init();
    app::setup_logger().expect("Failed to initialize logger");
}

// Runs before tests are run
#[cfg(test)]
#[ctor::ctor]
fn init_tests() {
    init();
}

fn main() {
    init();
    
    let explorers: Vec<Box<dyn ExplorerBuilder>> = vec![
        Box::new(explorers::ExampleExplorerBuilder::new()),
    ];

    let mut orchestrator = Orchestrator::new(
        OrchestratorMode::Auto,
        7,      // Provided number of planets
        explorers, // No explorers implemented yet
    )
    .unwrap_or_else(|e| {
        log::error!("Failed to create orchestrator: {e}");
        panic!("Failed to create orchestrator: {e}");
    });
    orchestrator.run().unwrap_or_else(|e| {
        log::error!("Orchestrator terminated with error: {e}");
    });
}

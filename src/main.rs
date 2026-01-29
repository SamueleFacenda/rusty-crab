mod app;
mod orchestrator;

use orchestrator::{Orchestrator, OrchestratorMode};

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

    let mut orchestrator = Orchestrator::new(
        OrchestratorMode::Auto,
        7,      // Provided number of planets
        vec![], // No explorers implemented yet
    )
    .unwrap_or_else(|e| {
        log::error!("Failed to create orchestrator: {e}");
        panic!("Failed to create orchestrator: {e}");
    });
    orchestrator.run().unwrap_or_else(|e| {
        log::error!("Orchestrator terminated with error: {e}");
    });
}

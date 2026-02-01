mod app;
mod explorers;
mod gui;
mod orchestrator;

pub(crate) use gui::{assets, events, game};
use orchestrator::{Orchestrator, OrchestratorMode};

use crate::explorers::{ExplorerBuilder, ExplorerFactory};
use crate::gui::run_gui;

fn init() {
    app::AppConfig::init();
    if !app::AppConfig::get().show_gui {
        // GUI has its own logger setup
        app::setup_logger().expect("Failed to initialize logger");
    }
}

// Runs before tests are run
#[cfg(test)]
#[ctor::ctor]
fn init_tests() { init(); }

fn main() {
    init();

    let config = app::AppConfig::get();

    if config.show_gui {
        run_gui().unwrap_or_else(|e| {
            log::error!("GUI terminated with error: {e}");
        });
        return;
    }

    let explorers = config.explorers.iter().map(ExplorerFactory::make_from_name).collect();

    let mut orchestrator = Orchestrator::new(
        OrchestratorMode::Auto,
        config.number_of_planets,
        explorers // No explorers implemented yet
    )
    .unwrap_or_else(|e| {
        log::error!("Failed to create orchestrator: {e}");
        panic!("Failed to create orchestrator: {e}");
    });
    orchestrator.run().unwrap_or_else(|e| {
        log::error!("Orchestrator terminated with error: {e}");
    });
}

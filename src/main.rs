mod orchestrator;
use orchestrator::{Orchestrator, ExampleExplorer, Explorer};
mod app;

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
    
    let mut orchestrator = Orchestrator::default();
    orchestrator.run();
}

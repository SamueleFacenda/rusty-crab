mod orchestrator;
use orchestrator::{Orchestrator, ExampleExplorer, Explorer};
mod app;

fn main() {
    app::AppConfig::init();
    app::setup_logger().expect("Failed to initialize logger");
    
    let mut orchestrator: Orchestrator<ExampleExplorer> = Orchestrator::default();
    
    orchestrator.run();
}
//! RustyCrab crate, contains our planet implementation and all the code of the project (it's private).
pub mod planet;
pub(crate) mod orchestrator;
pub(crate) mod app;


fn main() {
    app::AppConfig::init();
    app::setup_logger().expect("Failed to initialize logger");
    todo!("Run the orchestrator")
}
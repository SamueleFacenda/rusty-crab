use std::sync::OnceLock;

use clap::Parser;
use config::{Config, Environment, File};
use serde::Deserialize;

macro_rules! config_fields {
    ( $( $field:ident: $ty:ty = $default:expr ),* $(,)? ) => {
        #[allow(unused)]
        #[derive(Debug, Deserialize)]
        pub struct AppConfig {
            /// dependent crates will not be able to name this field.
            _priv: (),
            $( pub $field: $ty, )*
            pub log_level: String,
            pub log_file: Option<String>,
        }

        impl AppConfig {
            fn from_settings(settings: &Config, args: CliArgs) -> Self {
                Self {
                    _priv: (),
                    $( $field: settings.get(stringify!($field)).unwrap_or($default), )*
                    log_level: args.log_level,
                    log_file: args.log_file,
                }
            }
        }
    };
}

// Configuration fields with their default values
config_fields! {
    asteroid_probability: f32 = 0.01,
    sunray_probability: f32 =0.1,
    initial_asteroid_probability: f32 = 0.01,
    max_wait_time_ms: u64 = 2000,
    game_tick_seconds: f32 = 0.5,
    number_of_planets: usize = 7,
    explorers: Vec<String> = vec![],
    show_gui: bool = false,
    initial_planet_id: u32 = 1, // from 1 to <number_of_planets>
}

#[derive(Parser, Debug)]
#[command(name = "rusty_crab")]
pub struct CliArgs {
    /// Path to the config file
    #[arg(short, long, default_value = "config.toml")]
    pub config: String,
    /// Log level (error, warn, info, debug, trace, off)
    #[arg(long, default_value = "info")]
    pub log_level: String,
    /// Log file path
    #[arg(long)]
    pub log_file: Option<String>
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

impl AppConfig {
    pub fn init() {
        let args = CliArgs::parse();
        let settings = Config::builder()
            .add_source(File::with_name(&args.config).required(false))
            .add_source(Environment::with_prefix("RUSTY_CRAB").separator("_"))
            .build()
            .expect("Failed to build configuration"); // we cannot use logging here since it's not initialized yet
        CONFIG.set(AppConfig::from_settings(&settings, args)).expect("AppConfig can only be initialized once");
    }

    pub fn get() -> &'static AppConfig { CONFIG.get().expect("AppConfig is not initialized") }
}

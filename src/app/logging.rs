use super::AppConfig;
use fern::colors::ColoredLevelConfig;

pub fn setup_logger() -> Result<(), fern::InitError> {
    let log_level = &AppConfig::get().log_level;
    let colors = ColoredLevelConfig::new()
        .error(fern::colors::Color::Red)
        .warn(fern::colors::Color::Yellow)
        .info(fern::colors::Color::Blue)
        .debug(fern::colors::Color::Magenta)
        .trace(fern::colors::Color::White);


    let mut log = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}] [{}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                colors.color(record.level()),
                // record.target(),
                record.file().unwrap_or("<unknown>"),
                record.line().unwrap_or(0),
                message
            ))
        })
        .level(log_level.parse().unwrap_or_else(|e| {
            eprintln!("Failed to parse log level: {}. Defaulting to Info.", e);
            log::LevelFilter::Info
        }))
        .chain(std::io::stdout());

    if let Some(log_file) = &AppConfig::get().log_file {
        log = log.chain(fern::log_file(log_file)?);
    }

    log.apply()?;
    Ok(())
}
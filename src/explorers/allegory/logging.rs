use common_game::logging::ActorType::Explorer;
use common_game::logging::{Channel, EventType, LogEvent, Participant, Payload};
use common_game::utils::ID;
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

static LOGGER: AllegoryLogger = AllegoryLogger;

struct AllegoryLogger;

impl log::Log for AllegoryLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(LevelFilter::Trace))
}

pub fn explorer_log(id: ID, event_type: EventType, channel: Channel, payload: Payload) -> LogEvent {
    LogEvent::new(
        Some(Participant::new(Explorer, id)),
        None,
        event_type,
        channel,
        payload,
    )
}

pub fn emit_info(id: ID, s: String) {
    explorer_log(
        id,
        EventType::InternalExplorerAction,
        Channel::Info,
        Payload::from([("message".to_string(), s)])
    ).emit();
}

pub fn emit_warning(id: ID, s: String) {
    explorer_log(
        id,
        EventType::InternalExplorerAction,
        Channel::Warning,
        Payload::from([("warning".to_string(), s)])
    ).emit();
}

pub fn emit_error(id: ID, s: String) {
    explorer_log(
        id,
        EventType::InternalExplorerAction,
        Channel::Error,
        Payload::from([("error".to_string(), s)])
    ).emit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_game::logging::{ActorType, Channel, EventType};
    use std::collections::BTreeMap;

    #[test]
    fn test_new_explorer_log_structure() {
        let id = 42;
        let event_type = EventType::InternalExplorerAction;
        let channel = Channel::Info;
        let mut payload = BTreeMap::new();
        payload.insert("action".to_string(), "test".to_string());

        let event = explorer_log(id, event_type.clone(), channel.clone(), payload.clone());

        // Verify sender
        assert!(event.sender.is_some());
        let sender = event.sender.as_ref().unwrap();
        assert_eq!(sender.actor_type, ActorType::Explorer);
        assert_eq!(sender.id, id);

        // Verify receiver
        assert!(event.receiver.is_none());

        // other
        assert_eq!(event.event_type, event_type);
        assert_eq!(event.channel, channel);
        assert_eq!(event.payload, payload);
    }
}

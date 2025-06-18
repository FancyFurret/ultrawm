use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct UltraWMLogger {
    verbose: AtomicBool,
}

impl UltraWMLogger {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose: AtomicBool::new(verbose),
        }
    }

    fn format_log(&self, record: &Record) -> String {
        let (level_str, level_color) = match record.level() {
            Level::Error => ("[E]", "red"),
            Level::Warn => ("[W]", "yellow"),
            Level::Info => ("[I]", "green"),
            Level::Debug => ("[D]", "blue"),
            Level::Trace => ("[T]", "cyan"),
        };

        let target = if !record.target().is_empty() {
            let short_target = record
                .target()
                .split("::")
                .last()
                .unwrap_or(record.target());
            format!("[{}]", short_target.dimmed())
        } else {
            String::new()
        };

        let message = format!(
            "{} {}{} {}",
            level_str,
            target,
            if !target.is_empty() { " " } else { "" },
            record.args()
        );

        // Color the entire message for warnings and errors
        match record.level() {
            Level::Error => message.red().bold().to_string(),
            Level::Warn => message.yellow().bold().to_string(),
            _ => {
                // For other levels, only color the level indicator
                let colored_level = match level_color {
                    "green" => level_str.green().bold(),
                    "blue" => level_str.blue().bold(),
                    "cyan" => level_str.cyan().bold(),
                    _ => level_str.white().bold(),
                };
                message.replace(level_str, &colored_level.to_string())
            }
        }
    }
}

impl Log for UltraWMLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if self.verbose.load(Ordering::SeqCst) {
            metadata.level() <= Level::Trace
        } else {
            metadata.level() <= Level::Info
        }
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{}", self.format_log(record));
        }
    }

    fn flush(&self) {}
}

pub fn init_logger(verbose: bool) -> Result<(), log::SetLoggerError> {
    let logger = UltraWMLogger::new(verbose);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}

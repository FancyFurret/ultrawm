use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record};
use ultrawm_core::paths;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

pub struct UltraWMLogger {
    verbose: AtomicBool,
    log_file: Arc<Mutex<Option<File>>>,
}

impl UltraWMLogger {
    pub fn new(verbose: bool) -> Self {
        let log_path = paths::log_file_path();
        let log_file = if let Some(path) = &log_path {
            match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(path)
            {
                Ok(file) => Some(file),
                Err(e) => {
                    eprintln!("Warning: Failed to open log file at {:?}: {}", path, e);
                    None
                }
            }
        } else {
            None
        };

        Self {
            verbose: AtomicBool::new(verbose),
            log_file: Arc::new(Mutex::new(log_file)),
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

    fn format_log_plain(&self, record: &Record) -> String {
        let level_str = match record.level() {
            Level::Error => "[E]",
            Level::Warn => "[W]",
            Level::Info => "[I]",
            Level::Debug => "[D]",
            Level::Trace => "[T]",
        };

        let target = if !record.target().is_empty() {
            let short_target = record
                .target()
                .split("::")
                .last()
                .unwrap_or(record.target());
            format!("[{}]", short_target)
        } else {
            String::new()
        };

        format!(
            "{} {}{} {}",
            level_str,
            target,
            if !target.is_empty() { " " } else { "" },
            record.args()
        )
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
            let formatted = self.format_log(record);
            println!("{}", formatted);
            
            // Write to log file
            let plain_message = self.format_log_plain(record);
            if let Ok(mut file_opt) = self.log_file.lock() {
                if let Some(file) = file_opt.as_mut() {
                    let _ = writeln!(file, "{}", plain_message);
                    let _ = file.flush();
                }
            }
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

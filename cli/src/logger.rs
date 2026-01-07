use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use ultrawm_core::paths;

pub struct UltraWMLogger {
    quiet: AtomicBool,
    verbose: AtomicBool,
    log_file: Arc<Mutex<Option<File>>>,
    target_colors: Arc<Mutex<HashMap<String, usize>>>,
    next_color_index: AtomicUsize,
}

impl UltraWMLogger {
    pub fn new(quiet: bool, verbose: bool) -> Self {
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
            quiet: AtomicBool::new(quiet),
            verbose: AtomicBool::new(verbose),
            log_file: Arc::new(Mutex::new(log_file)),
            target_colors: Arc::new(Mutex::new(HashMap::new())),
            next_color_index: AtomicUsize::new(0),
        }
    }

    fn color_for_target(&self, target: &str) -> String {
        // Palette of colors that work well when dimmed
        let colors: &[fn(&str) -> ColoredString] = &[
            |s| s.green(),
            |s| s.yellow(),
            |s| s.blue(),
            |s| s.magenta(),
            |s| s.cyan(),
            |s| s.purple(),
        ];

        // Allocate a color index for this target if we haven't seen it before
        let color_index = {
            let mut target_colors = self.target_colors.lock().unwrap();
            target_colors
                .entry(target.to_string())
                .or_insert_with(|| {
                    let index = self.next_color_index.fetch_add(1, Ordering::SeqCst);
                    index % colors.len()
                })
                .clone()
        };

        let color_fn = colors[color_index];
        color_fn(target).to_string()
    }

    fn format_log(&self, record: &Record) -> String {
        let (level_str, level_color) = match record.level() {
            Level::Error => ("[E]", "red"),
            Level::Warn => ("[W]", "yellow"),
            Level::Info => ("[I]", "green"),
            Level::Debug => ("[D]", "blue"),
            Level::Trace => ("[T]", "white"),
        };

        let target = if !record.target().is_empty() {
            let short_target = record
                .target()
                .split("::")
                .last()
                .unwrap_or(record.target());
            let colored_target = self.color_for_target(short_target).dimmed();
            format!("[{}]", colored_target)
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
        let quiet = self.quiet.load(Ordering::SeqCst);
        let verbose = self.verbose.load(Ordering::SeqCst);

        if quiet {
            // Quiet mode: only Info, Warn, Error
            metadata.level() <= Level::Info
        } else if verbose {
            // Verbose mode: everything including Trace
            metadata.level() <= Level::Trace
        } else {
            // Default mode: up to Debug (no Trace)
            metadata.level() <= Level::Debug
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

pub fn init_logger(quiet: bool, verbose: bool) -> Result<(), log::SetLoggerError> {
    let logger = UltraWMLogger::new(quiet, verbose);
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}

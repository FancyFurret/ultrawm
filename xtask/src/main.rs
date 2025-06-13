use clap::{Parser, Subcommand};
use colored::*;
use std::process::{Command as ProcessCommand, ExitCode};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Tidy,
}

fn tidy() -> ExitCode {
    // Run cargo fmt
    let fmt_status = ProcessCommand::new("cargo")
        .args(["fmt", "--all"])
        .status()
        .expect("Failed to run cargo fmt");

    if !fmt_status.success() {
        println!("\n{}", "❌ Formatting failed".bold().red());
        return ExitCode::FAILURE;
    }

    // Run cargo fix
    let fix_status = ProcessCommand::new("cargo")
        .args(["fix", "--allow-dirty"])
        .status()
        .expect("Failed to run cargo fix");

    if !fix_status.success() {
        println!("\n{}", "❌ Cargo fix failed".bold().red());
        return ExitCode::FAILURE;
    }

    // Run cargo check with output redirected to null
    let check_output = ProcessCommand::new("cargo")
        .args(["check", "--message-format=json"])
        .output()
        .expect("Failed to run cargo check");

    // Count warnings from JSON output
    let output_str = String::from_utf8_lossy(&check_output.stdout);
    let warning_count = output_str
        .lines()
        .filter(|line| line.contains("\"level\":\"warning\""))
        .count();

    let separator = "=".repeat(50);
    println!("\n{}", separator.cyan());

    if warning_count > 0 {
        let width = 25;
        let border = "─".repeat(width);
        let message = format!("× Found {} issues", warning_count);
        let padding = (width - message.len()) / 2;
        let box_padding = (50 - width) / 2;
        let padding_str = " ".repeat(box_padding);

        let box_lines = [
            format!("{}┌{}┐", padding_str, border),
            format!(
                "{}│{}{}{}│",
                padding_str,
                " ".repeat(padding),
                message,
                " ".repeat(width - message.len() - padding + 1)
            ),
            format!("{}└{}┘", padding_str, border),
        ];

        println!("\n{}", box_lines.join("\n").bold().red());
        println!();
        ExitCode::FAILURE
    } else {
        let width = 25;
        let border = "─".repeat(width);
        let message = "✓ Success!";
        let padding = (width - message.len()) / 2;
        let box_padding = (50 - width) / 2;
        let padding_str = " ".repeat(box_padding);

        let box_lines = [
            format!("{}┌{}┐", padding_str, border),
            format!(
                "{}│{}{}{}│",
                padding_str,
                " ".repeat(padding),
                message,
                " ".repeat(width - message.len() - padding + 2)
            ),
            format!("{}└{}┘", padding_str, border),
        ];

        println!("\n{}", box_lines.join("\n").bold().green());
        println!();
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    match args.command {
        Command::Tidy => tidy(),
    }
}

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
        .args(["fix", "--allow-dirty", "--all-features", "--workspace"])
        .status()
        .expect("Failed to run cargo fix");

    if !fix_status.success() {
        println!("\n{}", "❌ Cargo fix failed".bold().red());
        return ExitCode::FAILURE;
    }

    // Run tests
    let test_output = ProcessCommand::new("cargo")
        .args(["test", "--all"])
        .output()
        .expect("Failed to run cargo test");

    if !test_output.status.success() {
        let output_str = String::from_utf8_lossy(&test_output.stdout);
        let stderr_str = String::from_utf8_lossy(&test_output.stderr);
        let combined_output = format!("{}{}", output_str, stderr_str);

        // Count failed tests by looking for the test summary line
        let failed_count = combined_output
            .lines()
            .find(|line| line.contains("test result: FAILED.") && line.contains("failed;"))
            .and_then(|line| {
                line.split(';')
                    .find(|part| part.trim().ends_with("failed"))
                    .and_then(|part| {
                        part.trim()
                            .split_whitespace()
                            .next()
                            .and_then(|num| num.parse::<usize>().ok())
                    })
            })
            .unwrap_or(0);

        let message = format!("× {} tests failed", failed_count);
        return print_result(&message, 50, false);
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

    if warning_count > 0 {
        let message = format!("× Found {} issues", warning_count);
        print_result(&message, 50, false)
    } else {
        let message = "✓ Success!";
        print_result(&message, 50, true)
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    match args.command {
        Command::Tidy => tidy(),
    }
}

fn print_result(message: &str, total_width: usize, success: bool) -> ExitCode {
    let separator = "=".repeat(50);
    println!("\n{}", separator.cyan());

    let box_width = 24;
    let border = "─".repeat(box_width);
    let padding = (box_width - message.len()) / 2;
    let box_padding = (total_width - box_width) / 2;
    let padding_str = " ".repeat(box_padding);

    let box_lines = [
        format!("{}┌{}┐", padding_str, border),
        format!(
            "{}│{}{}{}│",
            padding_str,
            " ".repeat(padding),
            message,
            " ".repeat(
                box_width - message.len() - padding + if message.len() % 2 == 0 { 2 } else { 1 }
            )
        ),
        format!("{}└{}┘", padding_str, border),
    ];

    box_lines.join("\n");

    if success {
        println!("\n{}", box_lines.join("\n").bold().green());
    } else {
        println!("\n{}", box_lines.join("\n").bold().red());
    }

    println!();
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

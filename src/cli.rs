use crate::tui;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "quest-log")]
#[command(about = "Quest Log - A gamified task tracker", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Serve,
    Ui,
}

pub async fn run_cli() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Ui) => {
            tui::run_tui().await?;
            Ok(false)
        }
        Some(Commands::Serve) => Ok(true),
    }
}

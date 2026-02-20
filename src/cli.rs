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
    /// Apply embedded migrations and exit (useful for CI/release hooks)
    MigrateOnly,
}

pub async fn run_cli() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Ui) => {
            tui::run_tui().await?;
            Ok(false)
        }
        Some(Commands::Serve) => Ok(true),
        Some(Commands::MigrateOnly) => {
            // Construct the database which will apply embedded migrations in Database::new()
            let _db = crate::database::Database::new().await?;
            println!("Migrations applied");
            Ok(false)
        }
    }
}

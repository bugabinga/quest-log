use crate::auth;
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
    /// Apply embedded migrations and exit (useful for CI/release hooks)
    MigrateOnly,
    /// Generate password hash for QUEST_LOG_EDITOR_PASSWORD_HASH env var
    EditorPassword,
}

pub async fn run_cli() -> Result<bool, Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Serve) => Ok(true),
        Some(Commands::MigrateOnly) => {
            // Construct the database which will apply embedded migrations in Database::new()
            let _db = crate::database::Database::new().await?;
            println!("Migrations applied");
            Ok(false)
        }
        Some(Commands::EditorPassword) => {
            run_editor_password_command().await?;
            Ok(false)
        }
    }
}

/// Interactive command to generate password hash for the editor
async fn run_editor_password_command() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Write};

    println!("\n=== Quest Log Editor Password Setup ===\n");

    // Read password securely
    print!("Enter a password for the editor: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    if password.is_empty() {
        eprintln!("Error: Password cannot be empty");
        std::process::exit(1);
    }

    // Confirm password
    print!("Confirm password: ");
    io::stdout().flush()?;
    let confirm = rpassword::read_password()?;

    if password != confirm {
        eprintln!("Error: Passwords do not match");
        std::process::exit(1);
    }

    // Generate hash
    let hash =
        auth::hash_password(&password).map_err(|e| format!("Failed to hash password: {}", e))?;

    println!("\n=== Generated Hash ===\n");
    println!("Add this to your environment:\n");
    println!("QUEST_LOG_EDITOR_PASSWORD_HASH={}", hash);
    println!("\n=== Example .env ===\n");
    println!("QUEST_LOG_EDITOR_PASSWORD_HASH={}", hash);

    Ok(())
}

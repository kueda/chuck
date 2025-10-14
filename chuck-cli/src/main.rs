use clap::{Parser, Subcommand, ValueEnum};

mod commands;
mod output;
mod progress;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Debug, ValueEnum)]
enum OutputFormat {
    /// CSV (default)
    Csv,
    /// DarwinCore Archive
    Dwc,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Csv
    }
}

#[derive(Clone, Debug, ValueEnum, PartialEq)]
pub enum DwcExtension {
    /// Simple Multimedia extension
    SimpleMultimedia,
    /// Audiovisual Media Description extension
    Audiovisual,
    /// Identifications extension
    Identifications,
}

impl From<DwcExtension> for chuck_core::DwcExtension {
    fn from(ext: DwcExtension) -> Self {
        match ext {
            DwcExtension::SimpleMultimedia => chuck_core::DwcExtension::SimpleMultimedia,
            DwcExtension::Audiovisual => chuck_core::DwcExtension::Audiovisual,
            DwcExtension::Identifications => chuck_core::DwcExtension::Identifications,
        }
    }
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Clear stored authentication token
    Clear,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with iNaturalist
    Auth {
        #[command(subcommand)]
        auth_command: Option<AuthCommands>,
    },
    /// Download iNaturalist observations
    Obs {
        /// Observations taxon (accepts name or ID)
        #[arg(short, long)]
        taxon: Option<String>,

        /// Observations place ID
        #[arg(short, long)]
        place_id: Option<i32>,

        /// Observations user (accepts username or ID)
        #[arg(short, long)]
        user: Option<String>,

        /// Observations earliest observation date, e.g. 2020-01-01
        #[arg(long)]
        d1: Option<String>,

        /// Observations latest observation date, e.g. 2020-01-01
        #[arg(long)]
        d2: Option<String>,

        /// Observations earliest creation date, e.g. 2020-01-01
        #[arg(long)]
        created_d1: Option<String>,

        /// Observations latest creation date, e.g. 2020-01-01
        #[arg(long)]
        created_d2: Option<String>,

        /// Path to write CSV if format is csv, path of DarwinCore Archive if
        /// format is dwc
        #[arg(long)]
        file: Option<String>,

        /// Fetch photos and include in a DarwinCore Archive
        #[arg(long)]
        fetch_photos: bool,

        #[arg(long, value_enum, default_value_t = OutputFormat::default())]
        format: OutputFormat,

        /// DarwinCore extenions to include when format is dwc
        #[arg(long = "dwc-ext", value_enum)]
        dwc_extensions: Vec<DwcExtension>,
    },
}

#[tokio::main(worker_threads = 5)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Auth { auth_command } => {
            match auth_command {
                Some(AuthCommands::Clear) => {
                    match chuck_core::auth::clear_auth_token() {
                        Ok(_) => println!("Authentication token cleared successfully!"),
                        Err(e) => eprintln!("Failed to clear token: {}", e),
                    }
                }
                None => {
                    match chuck_core::auth::authenticate_user().await {
                        Ok(_) => println!("Authentication successful!"),
                        Err(e) => eprintln!("Authentication failed: {}", e),
                    }
                }
            }
        }
        Commands::Obs {
            created_d1,
            created_d2,
            d1,
            d2,
            dwc_extensions,
            fetch_photos,
            file,
            format,
            place_id,
            taxon,
            user,
        } => commands::fetch_observations(
            file,
            taxon,
            place_id,
            user,
            d1,
            d2,
            created_d1,
            created_d2,
            fetch_photos,
            format,
            dwc_extensions
        ).await?,
    }
    Ok(())
}

use clap::{Parser, Subcommand, ValueEnum};
use chuck_core::auth::TokenStorage;

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
    /// Comments extension
    Comments,
}

impl From<DwcExtension> for chuck_core::DwcaExtension {
    fn from(ext: DwcExtension) -> Self {
        match ext {
            DwcExtension::SimpleMultimedia => chuck_core::DwcaExtension::SimpleMultimedia,
            DwcExtension::Audiovisual => chuck_core::DwcaExtension::Audiovisual,
            DwcExtension::Identifications => chuck_core::DwcaExtension::Identifications,
            DwcExtension::Comments => chuck_core::DwcaExtension::Comments,
        }
    }
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Clear stored authentication token
    Clear,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
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
                    match chuck_core::auth::StorageFactory::create() {
                        Ok(storage) => {
                            match storage.clear_token() {
                                Ok(_) => println!("Authentication token cleared successfully!"),
                                Err(e) => eprintln!("Failed to clear token: {e}"),
                            }
                        }
                        Err(e) => eprintln!("Storage error: {e}"),
                    }
                }
                None => {
                    match chuck_core::auth::StorageFactory::create_interactive() {
                        Ok(storage) => {
                            match chuck_core::auth::authenticate_user(&storage).await {
                                Ok(_) => println!("Authentication successful!"),
                                Err(e) => eprintln!("Authentication failed: {e}"),
                            }
                        }
                        Err(e) => eprintln!("Failed to initialize storage: {e}"),
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

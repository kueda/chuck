use inaturalist::models::Observation;
use super::ObservationWriter;
use crate::progress::ProgressManager;

pub enum CsvOutputStream {
    File(std::fs::File),
    Stdout(Box<dyn std::io::Write + Send>),
}

impl std::io::Write for CsvOutputStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            CsvOutputStream::File(file) => file.write(buf),
            CsvOutputStream::Stdout(stdout) => stdout.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            CsvOutputStream::File(file) => file.flush(),
            CsvOutputStream::Stdout(stdout) => stdout.flush(),
        }
    }
}

pub struct CsvOutput {
    writer: csv::Writer<CsvOutputStream>,
}

impl CsvOutput {
    pub fn new(file: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let header = [
            "id",
            "user_login",
            "taxon_name",
            "taxon_id",
            "latitude",
            "longitude",
            "private_latitude",
            "private_longitude",
            "positional_accuracy",
            "public_positional_accuracy",
            "obscured",
            "geoprivacy",
            "taxon_geoprivacy",
            "updated_at",
            "captive",
            "time_observed_at",
            "observed_on_string",
            "place_guess",
        ];
        let output_stream = if let Some(file_path) = file {
            // Create file and write header
            let mut wtr = csv::Writer::from_writer(CsvOutputStream::File(std::fs::File::create(&file_path)?));
            wtr.write_record(&header)?;
            wtr.flush()?;

            // Reopen file in append mode for actual writing
            CsvOutputStream::File(std::fs::OpenOptions::new()
                .append(true)
                .open(file_path)?)
        } else {
            // Write header to stdout
            let mut wtr = csv::Writer::from_writer(CsvOutputStream::Stdout(Box::new(std::io::stdout())));
            wtr.write_record(&header)?;
            wtr.flush()?;

            CsvOutputStream::Stdout(Box::new(std::io::stdout()))
        };

        Ok(Self {
            writer: csv::Writer::from_writer(output_stream),
        })
    }
}

impl ObservationWriter for CsvOutput {
    fn write_observations(
        &mut self,
        observations: &[Observation],
        progress_manager: &ProgressManager,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send {
        async move {
            for obs in observations {
                // Write the CSV
                let coords = obs.geojson
                    .as_ref()
                    .and_then(|geojson| geojson.coordinates.as_ref());
                let (mut lat, mut lng) = (String::new(), String::new());
                if let Some(coords) = coords && coords.len() >= 2 {
                    lat = coords[1].to_string();
                    lng = coords[0].to_string();
                }

                let private_coords = obs.private_geojson
                    .as_ref()
                    .and_then(|geojson| geojson.coordinates.as_ref());
                let (mut private_lat, mut private_lng) = (String::new(), String::new());
                if let Some(private_coords) = private_coords && private_coords.len() >= 2 {
                    private_lat = private_coords[1].to_string();
                    private_lng = private_coords[0].to_string();
                }

                self.writer.write_record(&[
                    obs.id.map_or(String::new(), |id| id.to_string()),
                    obs.user.as_ref().map_or(String::new(), |user| user.login.clone().unwrap_or_default()),
                    obs.taxon.as_ref().map_or(String::new(), |taxon| taxon.name.clone().unwrap_or_default()),
                    obs.taxon.as_ref().map_or(String::new(), |taxon| taxon.id.clone().unwrap_or_default().to_string()),
                    lat,
                    lng,
                    private_lat,
                    private_lng,
                    obs.positional_accuracy.as_ref().map_or(String::new(), |acc| acc.to_string()),
                    obs.public_positional_accuracy.as_ref().map_or(String::new(), |acc| acc.to_string()),
                    obs.obscured.map_or(String::new(), |obscured| obscured.to_string()),
                    obs.geoprivacy.as_ref()
                        .map_or(String::new(), |geoprivacy| geoprivacy.to_string()),
                    obs.taxon_geoprivacy.as_ref()
                        .map_or(String::new(), |taxon_geoprivacy| taxon_geoprivacy.to_string()),
                    obs.updated_at.clone().unwrap_or_default(),
                    obs.captive.map_or(String::new(), |captive| captive.to_string()),
                    obs.time_observed_at.clone().unwrap_or_default(),
                    obs.observed_on_string.clone().unwrap_or_default(),
                    obs.place_guess.clone().unwrap_or_default(),
                ])?;

                // If we're writing to stdout, just ignore the buffering and write each line as it gets processed
                if let CsvOutputStream::Stdout(_) = self.writer.get_ref() {
                    self.writer.flush()?;
                }
                progress_manager.observations_bar.inc(1);
            }
            self.writer.flush()?;
            Ok(())
        }
    }
}

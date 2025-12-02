pub mod csv;

use inaturalist::models::Observation;
use crate::progress::ProgressManager;

pub trait ObservationWriter: Send {
    fn write_observations(
        &mut self,
        observations: &[Observation],
        progress_manager: &ProgressManager,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    fn finalize(&mut self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send {
        async { Ok(()) }
    }
}

pub use csv::CsvOutput;

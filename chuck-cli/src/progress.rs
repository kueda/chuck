use indicatif::{ProgressBar, MultiProgress};

pub struct ProgressManager {
    pub multi: MultiProgress,
    pub observations_bar: ProgressBar,
    pub photos_bar: Option<ProgressBar>,
}

impl ProgressManager {
    pub fn new(show_progress: bool, fetch_photos: bool) -> Self {
        let multi = MultiProgress::new();

        let observations_bar = if show_progress {
            let bar = ProgressBar::new(10);
            bar.set_style(
                indicatif::ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:100.green/white} {pos:>7}/{len:7} observations ({eta}) {wide_msg}"
                )
                .unwrap()
                .progress_chars("██")
            );
            multi.add(bar.clone());
            bar
        } else {
            ProgressBar::hidden()
        };

        let photos_bar = if show_progress && fetch_photos {
            let bar = ProgressBar::new(0);
            bar.set_style(
                indicatif::ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:100.blue/white} {pos:>7}/{len:7} photos ({eta}) {wide_msg}"
                )
                .unwrap()
                .progress_chars("██")
            );
            multi.add(bar.clone());
            Some(bar)
        } else {
            None
        };

        Self {
            multi,
            observations_bar,
            photos_bar,
        }
    }

    pub fn set_observations_total(&self, total: u64) {
        self.observations_bar.set_length(total);
    }

    /// Prepare the photos progress bar to be incremented by a certain amount.
    /// If the bar's length is not long enough to receive that many
    /// increments, it will be embiggened
    pub fn prepare_photos_inc(&self, total_to_add: u64) {
        if let Some(ref bar) = self.photos_bar {
            let current_position = bar.position();
            let current_length = bar.length().unwrap_or(0);
            let remaining = current_length.saturating_sub(current_position);

            if remaining < total_to_add {
                let additional_length = total_to_add - remaining;
                bar.inc_length(additional_length);
            }
        }
    }
}

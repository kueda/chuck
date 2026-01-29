use indicatif::{ProgressBar, MultiProgress};

#[derive(Clone)]
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
}

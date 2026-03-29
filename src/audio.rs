#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct TimePoint {
    pub measure: i64,
    pub submeasure: f32,
}

impl Default for TimePoint {
    fn default() -> Self {
        Self {
            measure: 0,
            submeasure: 0.0,
        }
    }
}

impl From<TimePoint> for f32 {
    fn from(value: TimePoint) -> Self {
        value.measure as Self + value.submeasure
    }
}

impl TimePoint {
    pub fn new(measure: i64, submeasure: f32) -> Self {
        let measure = measure + submeasure.trunc() as i64;
        let submeasure = submeasure.fract();

        Self {
            measure,
            submeasure,
        }
    }

    pub fn ceil(&self) -> i64 {
        self.measure + self.submeasure.ceil() as i64
    }

    /// Get the sample index of a time point within a given channel.
    pub fn mono_sample_index(&self, channel_sample_rate: i32, bpm_changes: &[BPMChange]) -> usize {
        let time = ((self.measure * 4) as f32 + self.submeasure)
            * (60.0 / bpm_changes.first().expect("Fix this at some point").bpm);

        (time * (channel_sample_rate as f32)) as usize
    }

    fn normalise(&mut self) {
        while self.submeasure < 0.0 {
            self.measure -= 1;
            self.submeasure += 1.0;
        }

        while self.submeasure > 1.0 {
            self.measure += 1;
            self.submeasure -= 1.0;
        }

        if self.measure < 0 {
            *self = Self::default();
        }
    }
}

impl std::ops::Add for TimePoint {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let sub = self.submeasure + rhs.submeasure;

        let measure = self.measure + rhs.measure + sub.trunc() as i64;
        let submeasure = sub.fract();

        let mut tp = Self {
            measure,
            submeasure,
        };

        tp.normalise();
        tp
    }
}

impl std::ops::Sub for TimePoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let mut measure = self.measure - rhs.measure;
        let mut submeasure = self.submeasure - rhs.submeasure;

        measure -= submeasure.abs().ceil() as i64;
        submeasure += submeasure.abs().ceil();

        let mut tp = Self {
            measure,
            submeasure,
        };

        tp.normalise();
        tp
    }
}

impl std::ops::AddAssign for TimePoint {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for TimePoint {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
#[path = "./time_point_tests.rs"]
mod time_point_tests;

#[derive(Debug)]
pub struct BPMChange {
    pub time_point: TimePoint,
    pub bpm: f32,
}

#[derive(Debug)]
pub struct AudioFile {
    wav: wavers::Wav<f32>,
    samples: wavers::Samples<f32>,
}

impl AudioFile {
    pub fn new(mut wav: wavers::Wav<f32>) -> Result<Self, wavers::WaversError> {
        let samples = wav.read()?;
        Ok(Self { wav, samples })
    }

    pub fn sample_rate(&self) -> i32 {
        self.wav.sample_rate()
    }

    pub fn num_channels(&self) -> u16 {
        self.wav.n_channels()
    }

    pub fn num_samples(&self) -> usize {
        self.samples.len()
    }

    pub fn num_samples_per_channel(&self) -> usize {
        self.num_samples() / self.num_channels() as usize
    }

    pub fn draw(
        &self,
        num_samples: Option<usize>,
        rect: &egui::Rect,
        painter: &egui::Painter,
        stroke: egui::Stroke,
    ) {
        let num_drawn = num_samples
            .map(|v| v.min(self.wav.n_samples()))
            .unwrap_or_else(|| self.wav.n_samples());

        let points = self
            .samples
            .iter()
            .take(num_drawn)
            .enumerate()
            .map(|(i, sample)| {
                let tx = i as f32 / (num_drawn - 1) as f32;
                let ty = (sample + 1.0) / 2.0;
                egui::pos2(
                    rect.min.x + tx * rect.width(),
                    rect.min.y + ty * rect.height(),
                )
            })
            .collect::<Vec<_>>();

        if points.len() > 1 {
            painter.line(points, stroke);
        }
    }

    pub fn draw_channel(
        &self,
        channel_index: u16,
        num_points: Option<usize>,
        starting_sample: usize,
        num_samples: usize,
        rect: &egui::Rect,
        painter: &egui::Painter,
        stroke: egui::Stroke,
    ) {
        // Clamp to a normal amount
        let num_points = num_points
            .map(|v| v.min(self.num_samples_per_channel()))
            .unwrap_or_else(|| self.samples.len());

        let samples = (0..num_points).map(|i| {
            let ratio = (i as f32) / ((num_points - 1) as f32);

            let index = (ratio * (num_samples as f32)) as usize + starting_sample;

            // Adjust for channel sample
            self.samples
                .get(index * usize::from(self.num_channels()) + usize::from(channel_index))
        });

        let points = samples
            .into_iter()
            .take(num_points)
            .enumerate()
            .filter_map(|(i, sample)| {
                let sample = sample?;

                let tx = i as f32 / (num_points - 1) as f32;

                // [-1, 1] -> [0, 1], then invert for egui Y downwards
                let ty = 1.0 - f32::midpoint(*sample, 1.0);
                Some(egui::pos2(
                    rect.min.x + tx * rect.width(),
                    rect.min.y + ty * rect.height(),
                ))
            })
            .collect::<Vec<_>>();

        if points.len() > 1 {
            painter.line(points, stroke);
        }
    }
}

pub fn calculate_num_samples_all_channels(
    start: TimePoint,
    end: TimePoint,
    sample_rate: i32,
    num_channels: u16,
    bpm_changes: &[BPMChange],
) -> usize {
    let diff = end - start;

    let num_beats = 4.0 * (diff.measure as f32 + diff.submeasure);

    if bpm_changes.len() > 1 {
        todo!("Calculate num samples unimplemented for multi-bpm projects.");
    }

    let beat_length = 60.0 / bpm_changes.first().expect("this fo testing fool").bpm;

    let time_seconds = num_beats * beat_length;

    ((sample_rate as usize * num_channels as usize) as f32 * time_seconds).ceil() as usize
}

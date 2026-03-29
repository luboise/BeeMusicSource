#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Snapping {
    Measure(u16),
    Beat(u16),
}

impl Snapping {
    pub fn simplify(self) -> Self {
        match self {
            Self::Measure(m) => Self::Beat(m * 4),
            Self::Beat(b) => Self::Beat(b),
        }
    }

    pub fn as_measure_denom(self) -> u16 {
        match self {
            Self::Measure(m) => m,
            Self::Beat(b) => b * 4,
        }
    }
}

impl Default for Snapping {
    fn default() -> Self {
        Self::Measure(16)
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialOrd, PartialEq, Debug, Clone, Copy)]
pub struct TimePoint {
    pub measure: i64,
    pub submeasure: f64,
}

impl Default for TimePoint {
    fn default() -> Self {
        Self {
            measure: 0,
            submeasure: 0.0,
        }
    }
}

impl From<TimePoint> for f64 {
    fn from(value: TimePoint) -> Self {
        value.measure as Self + value.submeasure
    }
}

impl From<f64> for TimePoint {
    fn from(value: f64) -> Self {
        Self {
            measure: value.trunc() as i64,
            submeasure: value.fract(),
        }
    }
}

impl TimePoint {
    pub fn new(measure: i64, submeasure: f64) -> Self {
        let measure = measure + submeasure.trunc() as i64;
        let submeasure = submeasure.fract();

        Self {
            measure,
            submeasure,
        }
        .normalised()
    }

    pub fn clamped_to_zero(self) -> Self {
        if self.measure < 0 {
            Self::default()
        } else {
            self
        }
    }

    pub fn ratio(&self, other: &Self, ratio: impl Into<f64>) -> Self {
        let start = f64::from(*self);
        let end = f64::from(*other);

        Self::from(start + ratio.into() * (end - start))
    }

    pub fn get_ratio(&self, end: &Self, t: &Self) -> f64 {
        let start = f64::from(*self);
        let end = f64::from(*end);
        let t = f64::from(*t);

        (t - start) / (end - start)
    }

    pub fn ceil(&self) -> i64 {
        self.measure + self.submeasure.ceil() as i64
    }

    /// Get the sample index of a time point within a given channel.
    pub fn mono_sample_index(&self, channel_sample_rate: i32, bpm_changes: &[BPMChange]) -> usize {
        let time = ((self.measure * 4) as f64 + self.submeasure)
            * (60.0 / bpm_changes.first().expect("Fix this at some point").bpm);

        (time * (channel_sample_rate as f64)) as usize
    }

    pub fn normalise(&mut self) {
        while self.submeasure < 0.0 {
            self.measure -= 1;
            self.submeasure += 1.0;
        }

        while self.submeasure > 1.0 {
            self.measure += 1;
            self.submeasure -= 1.0;
        }
    }

    pub fn normalised(&self) -> Self {
        let mut ret = *self;
        ret.normalise();
        ret
    }

    pub fn quantise(&mut self, snapping: Snapping) {
        let beat_denom = match snapping {
            Snapping::Measure(v) => f64::from(v) / 4.0,
            Snapping::Beat(v) => f64::from(v),
        };

        let num_divisions = f64::from(*self) * beat_denom;

        *self = (num_divisions.round() / beat_denom).into();
    }

    pub fn quantised(mut self, snapping: Snapping) -> Self {
        self.quantise(snapping);
        self
    }
}

impl std::ops::Neg for TimePoint {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            measure: -self.measure,
            submeasure: -self.submeasure,
        }
        .normalised()
    }
}

impl std::ops::Add for TimePoint {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let sub = self.submeasure + rhs.submeasure;

        let measure = self.measure + rhs.measure + sub.trunc() as i64;
        let submeasure = sub.fract();

        Self {
            measure,
            submeasure,
        }
        .normalised()
    }
}

impl std::ops::Sub for TimePoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        let mut measure = self.measure - rhs.measure;
        let mut submeasure = self.submeasure - rhs.submeasure;

        measure -= submeasure.abs().ceil() as i64;
        submeasure += submeasure.abs().ceil();

        Self {
            measure,
            submeasure,
        }
        .normalised()
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
    pub bpm: f64,
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

    pub fn samples(&self) -> &[f32] {
        &self.samples
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
            let ratio = (i as f64) / ((num_points - 1) as f64);

            let index = (ratio * (num_samples as f64)) as usize + starting_sample;

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

                let tx = i as f64 / (num_points - 1) as f64;

                // [-1, 1] -> [0, 1], then invert for egui Y downwards
                let ty = 1.0 - f32::midpoint(*sample, 1.0);
                Some(egui::pos2(
                    rect.min.x + tx as f32 * rect.width(),
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

    let num_beats = 4.0 * (diff.measure as f64 + diff.submeasure);

    if bpm_changes.len() > 1 {
        todo!("Calculate num samples unimplemented for multi-bpm projects.");
    }

    let beat_length = 60.0 / bpm_changes.first().expect("this fo testing fool").bpm;

    let time_seconds = num_beats * beat_length;

    ((sample_rate as usize * num_channels as usize) as f64 * time_seconds).ceil() as usize
}

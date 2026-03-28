#[derive(Debug)]
pub struct AudioFile {
    pub samples: Vec<f32>,
    pub num_channels: usize,
    pub sample_rate: i32,
}

impl AudioFile {
    pub fn draw(
        &self,
        num_samples: Option<usize>,
        rect: &egui::Rect,
        painter: &egui::Painter,
        stroke: egui::Stroke,
    ) {
        let num_drawn = num_samples
            .map(|v| v.min(self.samples.len()))
            .unwrap_or_else(|| self.samples.len());

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

    pub fn draw_snapshot(
        &self,
        num_samples: Option<usize>,
        start_sample: usize,
        end_sample: usize,
        rect: &egui::Rect,
        painter: &egui::Painter,
        stroke: egui::Stroke,
    ) {
        let num_drawn = num_samples
            .map(|v| v.min(self.samples.len()))
            .unwrap_or_else(|| self.samples.len());

        let samples = (0..num_drawn)
            .map(|i| {
                let ratio = (i as f32) / ((num_drawn - 1) as f32);

                let index = (((end_sample - start_sample) as f32) * ratio) as usize + start_sample;

                self.samples[index]
            })
            .collect::<Vec<_>>();

        let points = samples
            .into_iter()
            .take(num_drawn)
            .enumerate()
            .map(|(i, sample)| {
                let tx = i as f32 / (num_drawn - 1) as f32;
                let ty = f32::midpoint(sample, 1.0);
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
}

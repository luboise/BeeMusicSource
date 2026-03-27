#[derive(Debug)]
pub struct AudioFile {
    pub samples: Vec<f32>,
    pub num_channels: usize,
    pub sample_rate: i32,
}

impl AudioFile {
    pub fn draw(
        &self,
        num_samples: usize,
        rect: &egui::Rect,
        painter: &egui::Painter,
        stroke: egui::Stroke,
    ) {
        let num_drawn = num_samples.min(self.samples.len());

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
}

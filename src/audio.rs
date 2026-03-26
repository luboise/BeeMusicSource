#[derive(Debug)]
pub struct AudioFile {
    pub samples: Vec<f32>,
    pub num_channels: usize,
    pub sample_rate: i32,
}

impl AudioFile {
    pub fn draw(&self, rect: &egui::Rect, painter: &egui::Painter, stroke: egui::Stroke) {
        let points = self
            .samples
            .iter()
            .enumerate()
            .map(|(i, sample)| {
                let tx = i as f32 / (self.samples.len() - 1) as f32;
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

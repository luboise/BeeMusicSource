use crate::audio::calculate_num_samples_all_channels;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct JonnahSlicer<'a> {
    project_path: Option<std::path::PathBuf>,
    #[serde(skip)]
    project: LiveProject,

    display_start: crate::audio::TimePoint,

    /// The number of points to draw in channel
    visual_density: usize,

    slice_snapping: crate::audio::Snapping,

    zoom_level: f32,

    #[serde(skip)]
    jonnah_image: Option<egui::Image<'a>>,
    #[serde(skip)]
    drag_and_drop: egui::DragAndDrop,
    #[serde(skip)]
    audio_player: Option<crate::audio_player::AudioPlayer>,
}

#[derive(Debug)]
struct LiveStem {
    stem: crate::bms::Stem,
    audio: Option<crate::audio::AudioFile>,
}

#[derive(Debug)]
pub struct LiveProject {
    stems: Vec<LiveStem>,
    bpm_changes: Vec<crate::audio::BPMChange>,
}

impl LiveProject {
    pub fn as_project(&self) -> crate::bms::Project {
        crate::bms::Project {
            stems: self.stems.iter().map(|stem| stem.stem.clone()).collect(),
        }
    }
}

impl Default for LiveProject {
    fn default() -> Self {
        Self {
            stems: Default::default(),
            bpm_changes: vec![crate::audio::BPMChange {
                time_point: crate::audio::TimePoint::default(),
                bpm: 160.0,
            }],
        }
    }
}

impl Default for JonnahSlicer<'_> {
    fn default() -> Self {
        Self {
            // Example stuff:
            project: LiveProject::default(),
            visual_density: 6000,
            zoom_level: 1.0,
            jonnah_image: None,
            display_start: crate::audio::TimePoint::default(),
            slice_snapping: crate::audio::Snapping::default(),
            audio_player: crate::audio_player::AudioPlayer::new().ok(),
            project_path: None,
            drag_and_drop: egui::DragAndDrop::default(),
        }
    }
}

impl JonnahSlicer<'_> {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }

    pub fn new_from_args(audio_path: Option<std::path::PathBuf>) -> Self {
        let Some(audio_path) = audio_path else {
            return Default::default();
        };

        Self {
            project: LiveProject {
                stems: vec![
                    LiveStem {
                        stem: crate::bms::Stem {
                            audio_path: audio_path.clone(),
                            slices: vec![],
                        },
                        audio: None,
                    },
                    LiveStem {
                        stem: crate::bms::Stem {
                            audio_path,
                            slices: vec![],
                        },
                        audio: None,
                    },
                ],

                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl eframe::App for JonnahSlicer<'_> {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.jonnah_image.is_none() {
            egui_extras::install_image_loaders(ctx);
            self.jonnah_image = Some(
                egui::Image::new(egui::include_image!("../assets/jonnah.jpg"))
                    .corner_radius(5.0)
                    .tint(egui::Color32::WHITE),
            );
        }

        let (dropped, hovered) =
            ctx.input(|i| (i.raw.dropped_files.clone(), i.raw.hovered_files.clone()));

        if !hovered.is_empty() {
            egui::Panel::left("File Hover Preview").show(ctx, |ui| {
                if !hovered.is_empty() {
                    for path in hovered.into_iter().filter_map(|file| file.path) {
                        ui.label(path.display().to_string());
                    }
                }
            });
        }

        for dropped_file in dropped {
            if let Some(path) = dropped_file.path {
                self.project.stems.push(LiveStem {
                    stem: crate::bms::Stem {
                        audio_path: path.clone(),
                        slices: vec![],
                    },
                    audio: None,
                });
            }
        }

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // TODO: Make this periodic
        self.project.stems.iter_mut().for_each(|st| {
            if st.audio.is_none()
                && let Ok(wav) = wavers::Wav::from_path(&st.stem.audio_path)
            {
                // TODO: FIX THIS
                st.audio = Some(crate::audio::AudioFile::new(wav).expect("Bad wav"));
            }
        });

        let scroll_delta = ctx.input(|i| i.smooth_scroll_delta);
        let home_pressed = ctx.input(|i| i.key_pressed(egui::Key::Home));

        const SCROLL_SENSITIVITY: f64 = 0.3;
        if scroll_delta.y != 0.0 {
            self.display_start = (self.display_start
                + crate::audio::TimePoint::new(0, -scroll_delta.y as f64 * SCROLL_SENSITIVITY))
            .clamped_to_zero();
        }

        if home_pressed {
            self.display_start = Default::default();
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save").clicked() {
                            serde_json::to_writer_pretty(
                                std::fs::OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .truncate(true)
                                    .open("./project.jonnah")
                                    .expect("Failed to open jonnah???"),
                                &self.project.as_project(),
                            )
                            .expect("Failed to serialize jonnah");
                        }

                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });

            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.label("Snapping: ");
                for snap_v in [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 64, 128] {
                    let button = ui.button(format!("1/{snap_v}"));

                    if self.slice_snapping.as_measure_denom() == snap_v {
                        button.highlight();
                    } else {
                        if button.clicked() {
                            self.slice_snapping = crate::audio::Snapping::Measure(snap_v);
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(jonnah) = &self.jonnah_image {
                jonnah.paint_at(ui, ui.content_rect());
            }

            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!("JonnahSlicer v{}", env!("CARGO_PKG_VERSION")));

            ui.add(egui::Slider::new(&mut self.zoom_level, 0.0..=8.0).text("Zoom"));

            ui.separator();

            let display_length = (8.0 * self.zoom_level) as i64;

            let stroke = egui::Stroke::new(1.5, egui::Color32::LIGHT_BLUE);

            egui::ScrollArea::vertical().show(ui, |ui| {
                let end_time_point =
                    self.display_start + crate::audio::TimePoint::new(display_length, 0.0);

                {
                    // Draw measures labels
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::Vec2::new(ui.available_width(), 20.0),
                        egui::Sense::click(),
                    );

                    let start = f64::from(self.display_start);
                    let end = (self.display_start.measure + display_length) as f64;

                    let mut i = self.display_start.ceil() as f64;
                    while i < end {
                        let tx = (i - start) / (end - start);
                        let pos = egui::pos2(rect.min.x + tx as f32 * rect.width(), rect.min.y);

                        ui.put(
                            egui::Rect::from_pos(pos).expand(20.0),
                            egui::Label::new(i.to_string()),
                        );

                        i += 1.0;
                    }
                }

                let (mouse_pos, lmb_down, rmb_down) = ctx.input(|i| {
                    (
                        i.pointer.latest_pos(),
                        i.pointer.button_down(egui::PointerButton::Primary),
                        i.pointer.button_down(egui::PointerButton::Secondary),
                    )
                });

                for stem in &mut self.project.stems {
                    let num_channels = stem.audio.as_ref().map(|v| v.num_channels()).unwrap_or(1);

                    let mut first_rect: Option<egui::Rect> = None;

                    const RECT_HEIGHT: f32 = 200.0;

                    let slices = stem
                        .stem
                        .slices
                        .iter()
                        .filter_map(|slice| {
                            (self.display_start..=end_time_point)
                                .contains(&slice.time_point)
                                .then_some(slice.clone())
                        })
                        .collect::<Vec<_>>();

                    for channel_index in 0..num_channels {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::Vec2::new(ui.available_width(), RECT_HEIGHT),
                            egui::Sense::click(),
                        );

                        if first_rect.is_none() {
                            first_rect.replace(rect);
                        }

                        let mouse_x_ratio = {
                            let first_rect =
                                first_rect.as_ref().expect("This has been checked already");
                            let x1 = first_rect.min.x;
                            let x2 = first_rect.max.x;

                            ((mouse_pos.unwrap_or_default().x - x1) / (x2 - x1)).clamp(0.0, 1.0)
                        };

                        if let Some(mouse_pos) = &mouse_pos
                            && rect.contains(*mouse_pos)
                        {
                            // if response.clicked()
                            if lmb_down {
                                stem.stem.slices.push(crate::bms::Slice {
                                    time_point: self
                                        .display_start
                                        .ratio(&end_time_point, mouse_x_ratio)
                                        .quantised(self.slice_snapping),
                                });

                                stem.stem.slices.dedup_by_key(|v| v.time_point);
                                stem.stem.slices.sort_by_key(|v| v.time_point);

                            // } else if response.secondary_clicked()
                            } else if rmb_down {
                                let ratiod =
                                    self.display_start.ratio(&end_time_point, mouse_x_ratio);

                                const DELETE_DISTANCE: f64 = 0.15;

                                stem.stem.slices.retain(|slice| {
                                    f64::from(slice.time_point - ratiod).abs() > DELETE_DISTANCE
                                });
                            }
                        }

                        if let Some(audio) = &stem.audio {
                            if response.middle_clicked()
                                && let Some(audio_player) = &mut self.audio_player
                            {
                                let cursor_time_point =
                                    self.display_start.ratio(&end_time_point, mouse_x_ratio);

                                stem.stem.slices.sort_by_key(|v| v.time_point);

                                // If there is a slice before our cursor
                                if let Some((first_slice_index, first_slice)) = stem
                                    .stem
                                    .slices
                                    .iter()
                                    .enumerate()
                                    .rfind(|(_, slice)| slice.time_point < cursor_time_point)
                                    && let Some(second_slice) =
                                        stem.stem.slices.get(first_slice_index + 1)
                                {
                                    let start = first_slice.time_point;
                                    let end = second_slice.time_point;

                                    let start_sample_index = start.mono_sample_index(
                                        audio.sample_rate(),
                                        &self.project.bpm_changes,
                                    );
                                    let end_sample_index = end.mono_sample_index(
                                        audio.sample_rate(),
                                        &self.project.bpm_changes,
                                    );

                                    audio_player.add_audio(
                                        &stem.audio.as_ref().expect("NO AUDIO IN STEM?").samples()
                                            [start_sample_index * audio.num_channels() as usize
                                                ..end_sample_index * audio.num_channels() as usize],
                                    );
                                }
                            }

                            let painter = ui.painter_at(rect);
                            painter.rect_filled(rect, 0.0, egui::Color32::DARK_GRAY);

                            let num_samples = calculate_num_samples_all_channels(
                                self.display_start,
                                self.display_start
                                    + crate::audio::TimePoint::new(display_length, 0.0),
                                audio.sample_rate(),
                                audio.num_channels(),
                                &self.project.bpm_changes,
                            );

                            let starting_sample = self
                                .display_start
                                .mono_sample_index(audio.sample_rate(), &self.project.bpm_changes);

                            audio.draw_channel(
                                channel_index,
                                Some(self.visual_density),
                                starting_sample,
                                num_samples / usize::from(num_channels),
                                &rect,
                                &painter,
                                stroke,
                            );

                            for slice in &slices {
                                let ratio = self
                                    .display_start
                                    .get_ratio(&end_time_point, &slice.time_point);

                                let tx = rect.min.x + (ratio as f32) * (rect.max.x - rect.min.x);

                                let points = [
                                    egui::Pos2 {
                                        x: tx,
                                        y: rect.min.y,
                                    },
                                    egui::Pos2 {
                                        x: tx,
                                        y: rect.max.y,
                                    },
                                ];

                                painter.line_segment(
                                    points,
                                    egui::Stroke::new(5.0, egui::Color32::BLACK),
                                );
                            }
                        }
                    }
                }
            });

            let rect = ui.available_rect_before_wrap();

            // Allocate it so its being used
            ui.allocate_rect(rect, egui::Sense::all());

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}

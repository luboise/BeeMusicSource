#[derive(Default, Debug)]
enum ProjectStatus {
    #[default]
    None,
    Loaded,
    Failed(Box<dyn std::error::Error>),
}

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
    project_status: ProjectStatus,

    #[serde(skip)]
    jonnah_image: Option<egui::Image<'a>>,
    #[serde(skip)]
    drag_and_drop: egui::DragAndDrop,
    #[serde(skip)]
    audio_player: Option<crate::audio_player::AudioPlayer>,
}

#[derive(Debug)]
struct LiveStem {
    stem: crate::project::Stem,
    audio: Option<crate::audio::AudioFile>,
}

impl From<crate::project::Stem> for LiveStem {
    fn from(stem: crate::project::Stem) -> Self {
        Self { stem, audio: None }
    }
}

#[derive(Debug)]
pub struct LiveProject {
    stems: Vec<LiveStem>,
    bpm_changes: Vec<crate::audio::BPMChange>,
}

impl std::convert::TryFrom<crate::project::Project> for LiveProject {
    type Error = Box<dyn std::error::Error>;

    fn try_from(project: crate::project::Project) -> Result<Self, Self::Error> {
        Ok(Self {
            stems: project.stems.into_iter().map(|v| v.into()).collect(),
            bpm_changes: project.bpm_changes,
        })
    }
}

impl LiveProject {
    pub fn as_project(&self) -> crate::project::Project {
        crate::project::Project {
            stems: self.stems.iter().map(|stem| stem.stem.clone()).collect(),
            bpm_changes: self.bpm_changes.clone(),
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
            project_status: Default::default(),
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
            let mut x: JonnahSlicer<'_> =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            if x.project_path.is_none() {
                x.project_path = Some(std::path::PathBuf::from("./projects/new_project/"));
            }

            x
        } else {
            Default::default()
        }
    }

    // TODO: Document errors that this function can return
    pub fn save_to_disk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        crate::project::save_project(
            &self.project.as_project(),
            self.project_path
                .clone()
                .map(crate::project::normalise_project_path)
                .unwrap_or_else(|| std::path::PathBuf::from("./project.jonnah")),
        )
    }
}

impl eframe::App for JonnahSlicer<'_> {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.project_path.is_some() {
            match &self.project_status {
                ProjectStatus::None => {
                    self.project =
                        crate::project::load_project(self.project_path.as_ref().unwrap())
                            .and_then(|project| project.try_into())
                            .unwrap_or_default();

                    self.project_status = ProjectStatus::Loaded;
                }
                ProjectStatus::Failed(_error) => (),
                ProjectStatus::Loaded => (),
            }
        }

        if ctx.input_mut(|ui| {
            ui.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::CTRL,
                egui::Key::S,
            ))
        }) {
            self.save_to_disk();
        }

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
                let parent_dir = std::env::current_dir();

                let audio_path = parent_dir
                    .ok()
                    .and_then(|parent| pathdiff::diff_paths(&path, parent))
                    .unwrap_or_else(|| path.canonicalize().unwrap_or(path));

                self.project.stems.push(LiveStem {
                    stem: crate::project::Stem {
                        audio_path,
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
                            self.save_to_disk();
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

            egui::ScrollArea::vertical().show(ui, |ui| {
                let end_time_point =
                    self.display_start + crate::audio::TimePoint::new(display_length, 0.0);

                // Draw measures labels
                {
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

                for stem in &mut self.project.stems {
                    let (rect, event) = draw_stem(
                        stem,
                        &self.project.bpm_changes,
                        ctx,
                        ui,
                        self.display_start,
                        end_time_point,
                    );

                    match event {
                        Some(StemEvent::PlayAudio(time_point)) if let Some(audio) = &stem.audio => {
                            stem.stem.slices.sort_by_key(|v| v.time_point);

                            // If there is a slice before our cursor
                            if let Some((first_slice_index, first_slice)) = stem
                                .stem
                                .slices
                                .iter()
                                .enumerate()
                                .rfind(|(_, slice)| slice.time_point < time_point)
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

                                // TODO: Put make this pre-trim it before fetching the channels?
                                let channels = stem
                                    .audio
                                    .as_ref()
                                    .expect("NO AUDIO IN STEM?")
                                    .channels()
                                    .into_iter()
                                    .map(|channel| {
                                        channel
                                            .get(start_sample_index..end_sample_index)
                                            .expect("bad channel")
                                            .to_vec()
                                    })
                                    .collect::<Vec<_>>();

                                let playback = crate::audio_player::AudioPlayback::new(
                                    // TODO: Make this not clone the channels completely
                                    channels, None,
                                )
                                .expect("failed to add audio");

                                self.audio_player.as_ref().unwrap().add_audio(playback);
                            }
                        }
                        Some(StemEvent::LeftClick(time_point)) => {
                            stem.stem.slices.push(crate::project::Slice {
                                time_point: time_point.quantised(self.slice_snapping),
                            });
                        }

                        Some(StemEvent::RightClick(time_point)) => {
                            stem.stem.slices.dedup_by_key(|v| v.time_point);
                            stem.stem.slices.sort_by_key(|v| v.time_point);

                            const DELETE_DISTANCE: f64 = 0.15;

                            stem.stem.slices.retain(|slice| {
                                f64::from(slice.time_point - time_point).abs() > DELETE_DISTANCE
                            });
                        }
                        Some(StemEvent::PlayAudio(_)) | None => (),
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

enum StemEvent {
    LeftClick(crate::audio::TimePoint),
    RightClick(crate::audio::TimePoint),
    PlayAudio(crate::audio::TimePoint),
}

#[must_use]
fn draw_stem(
    live_stem: &LiveStem,
    bpm_changes: &[crate::audio::BPMChange],
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    display_start: crate::audio::TimePoint,
    end: crate::audio::TimePoint,
) -> (egui::Rect, Option<StemEvent>) {
    const RECT_HEIGHT: f32 = 200.0;

    let (mouse_pos, lmb_down, rmb_down) = ctx.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.button_down(egui::PointerButton::Primary),
            i.pointer.button_down(egui::PointerButton::Secondary),
        )
    });

    let (rect, response) = ui.allocate_exact_size(
        egui::Vec2::new(ui.available_width(), RECT_HEIGHT),
        egui::Sense::click(),
    );
    let mouse_x_ratio = {
        let x1 = rect.min.x;
        let x2 = rect.max.x;

        ((mouse_pos.unwrap_or_default().x - x1) / (x2 - x1)).clamp(0.0, 1.0)
    };

    let stroke = egui::Stroke::new(3.0, egui::Color32::WHITE);

    let painter = ui.painter_at(rect);

    let slices = live_stem
        .stem
        .slices
        .iter()
        .filter_map(|slice| {
            (display_start..=end)
                .contains(&slice.time_point)
                .then_some(slice.clone())
        })
        .collect::<Vec<_>>();

    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(35));
    for slice in &slices {
        let ratio = display_start.get_ratio(&end, &slice.time_point);

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

        painter.line_segment(points, stroke);

        for i in display_start.measure..end.measure {
            let ratio = display_start.get_ratio(&end, &crate::audio::TimePoint::new(i, 0.0));

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

            let measure_stroke =
                egui::Stroke::new(2.0, egui::Color32::DARK_BLUE.linear_multiply(0.7));
            painter.line_segment(points, measure_stroke);
        }
    }

    let mut event = None;

    if let Some(mouse_pos) = &mouse_pos
        && rect.contains(*mouse_pos)
    {
        let click_point = display_start.ratio(&end, mouse_x_ratio);

        if lmb_down {
            event = Some(StemEvent::LeftClick(click_point));
        } else if rmb_down {
            event = Some(StemEvent::RightClick(click_point));
        }
    }

    if let Some(audio) = &live_stem.audio {
        let num_channels = audio.num_channels();

        let display_length = (end - display_start).ceil();

        let num_samples = crate::audio::calculate_num_samples_all_channels(
            display_start,
            end,
            audio.sample_rate(),
            num_channels,
            bpm_changes,
        )
        .unwrap();

        let starting_sample = display_start.mono_sample_index(audio.sample_rate(), bpm_changes);

        // TODO: Move this somewhere else?
        let visual_density = 6000;

        let waveform_stroke =
            egui::Stroke::new(1.5, egui::Color32::from_gray(190).linear_multiply(0.7));

        audio.draw_channel(
            0,
            Some(visual_density),
            starting_sample,
            num_samples / usize::from(num_channels),
            &rect,
            &painter,
            waveform_stroke,
        );

        if response.middle_clicked() || ui.input(|input| input.key_pressed(egui::Key::G)) {
            event = Some(StemEvent::PlayAudio(
                display_start.ratio(&end, mouse_x_ratio),
            ));
        }
    }

    (rect, event)
}

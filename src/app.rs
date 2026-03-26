/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct BeeMusicSource {
    // Example stuff:
    label: String,

    #[serde(skip)]
    project: LiveProject,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,
}

#[derive(Debug)]
struct LiveStem {
    stem: crate::bms::Stem,
    audio: Option<crate::audio::AudioFile>,
}

#[derive(Debug, Default)]
pub struct LiveProject {
    stems: Vec<LiveStem>,
}

impl Default for BeeMusicSource {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            project: LiveProject::default(),
            value: 2.7,
        }
    }
}

impl BeeMusicSource {
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
        if let Some(audio_path) = audio_path {
            Self {
                project: LiveProject {
                    stems: vec![LiveStem {
                        stem: crate::bms::Stem {
                            audio_path,
                            slices: vec![],
                        },
                        audio: None,
                    }],
                },
                ..Default::default()
            }
        } else {
            Default::default()
        }
    }
}

impl eframe::App for BeeMusicSource {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // TODO: Make this periodic
        self.project.stems.iter_mut().for_each(|st| {
            if st.audio.is_none()
                && let Ok((samples, sample_rate)) = wavers::read(&st.stem.audio_path)
            {
                st.audio = Some(crate::audio::AudioFile {
                    samples: samples.to_vec(),
                    num_channels: 2,
                    sample_rate,
                });
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            let stroke = egui::Stroke::new(1.5, egui::Color32::LIGHT_BLUE);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for stem in &self.project.stems {
                    let (rect, _response) = ui.allocate_exact_size(
                        egui::Vec2::new(ui.available_width(), 200.0),
                        egui::Sense::hover(),
                    );

                    if let Some(audio) = &stem.audio {
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 0.0, egui::Color32::DARK_GRAY);

                        audio.draw(&rect, &painter, stroke);
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

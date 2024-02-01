use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs::{self, read_to_string},
    ops::RangeInclusive,
};

use colcon::{srgb_to_irgb, Space};

use eframe::{
    egui::{
        self, CentralPanel, Context, DragValue, Frame, Grid, Label, Rgba, RichText, ScrollArea,
        Sense, SidePanel, Widget,
    },
    emath::Align2,
    epaint::{Color32, Rounding, Stroke},
    App, CreationContext,
};

mod lch;
use lch::LCH;
use rfd::FileDialog;

use super::{Collurgy, Exporter};

const LI: &'static str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

fn scale_factor() -> f32 {
    if let Ok(val) = env::var("GDK_DPI_SCALE") {
        val.parse::<f32>().expect("Bad GDK_DPI_SCALE value")
    } else if let Ok(val) = env::var("GDK_SCALE") {
        val.parse::<f32>().expect("Bad GDK_SCALE value")
    } else {
        1.0
    }
}

// ColorButton {{{
struct ColorButton {
    text: String,
    color: Color32,
    fill: Color32,
    font_size: f32,
}

impl ColorButton {
    pub fn new(text: impl ToString, color: Color32, fill: Color32, font_size: f32) -> Self {
        Self {
            text: text.to_string(),
            color,
            fill,
            font_size,
        }
    }
}

impl Widget for ColorButton {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.allocate_response(ui.available_size(), Sense::click());
        if response.is_pointer_button_down_on() {
            (self.color, self.fill) = (self.fill, self.color);
        }
        if response.hovered() {
            ui.painter()
                .rect(response.rect, Rounding::ZERO, self.color, Stroke::NONE);
            ui.painter().rect(
                response.rect.shrink2(
                    (
                        response.rect.width() * 0.025,
                        response.rect.height() * 0.025,
                    )
                        .into(),
                ),
                Rounding::ZERO,
                self.fill,
                Stroke::NONE,
            );
        } else {
            ui.painter()
                .rect(response.rect, Rounding::ZERO, self.fill, Stroke::NONE);
        }
        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            self.text,
            eframe::epaint::FontId::proportional(self.font_size),
            self.color,
        );
        response
    }
}
// ColorButton }}}

// ColorScale {{{
struct ColorScale<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    round: f32,
    text: String,
    color: Color32,
    fill: Color32,
    font_size: f32,
}

impl<'a> ColorScale<'a> {
    pub fn new(
        value: &'a mut f32,
        range: RangeInclusive<f32>,
        round: f32,
        text: impl ToString,
        color: Color32,
        fill: Color32,
        font_size: f32,
    ) -> Self {
        Self {
            value,
            range,
            round,
            text: text.to_string(),
            color,
            fill,
            font_size,
        }
    }
}

impl<'a> Widget for ColorScale<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.allocate_response(ui.available_size(), Sense::click_and_drag());
        if response.dragged() {
            if let Some(pos) = response.hover_pos() {
                *self.value = ((pos.x - response.rect.left())
                    / (response.rect.right() - response.rect.left()))
                    * (self.range.end() - self.range.start())
                    + self.range.start();
                *self.value = ((*self.value * (1.0 / self.round)).round() * self.round)
                    .clamp(*self.range.start(), *self.range.end())
            }
        }

        let (r1, r2) = response.rect.split_left_right_at_fraction(
            (*self.value - self.range.start()) / (self.range.end() - self.range.start()),
        );

        ui.set_clip_rect(r1);

        ui.painter()
            .rect(response.rect, Rounding::ZERO, self.color, Stroke::NONE);

        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            &self.text,
            eframe::epaint::FontId::proportional(self.font_size),
            self.fill,
        );

        ui.set_clip_rect(r2);

        ui.painter()
            .rect(response.rect, Rounding::ZERO, self.fill, Stroke::NONE);

        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            self.text,
            eframe::epaint::FontId::proportional(self.font_size),
            self.color,
        );

        response
    }
}
// ColorScale }}}

pub enum Output {
    Exporter(String),
    JSON,
    TOML,
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Output::Exporter(s) => f.write_fmt(format_args!("Export/{}", s)),
            Output::JSON => f.write_str("Save/JSON"),
            Output::TOML => f.write_str("Save/TOML"),
        }
    }
}

pub struct CollurgyUI {
    data: Collurgy,
    exporters: HashMap<String, Exporter>,
    output: Output,
    scale: f32,
}

impl CollurgyUI {
    // {{{
    pub fn new(
        _cc: &CreationContext,
        mut data: Collurgy,
        exporters: HashMap<String, Exporter>,
    ) -> Self {
        // Fill missing extras into data
        for (k, v) in exporters.iter() {
            if let Some(extras) = &v.extras {
                if !data.extras.contains_key(k) {
                    data.extras.insert(k.to_string(), extras.clone());
                }
            }
        }
        Self {
            data,
            output: Output::TOML,
            exporters,
            scale: scale_factor(),
        }
    }
    fn process_output(&self) -> String {
        match &self.output {
            Output::Exporter(s) => self.exporters[s].export(&self.data),
            Output::JSON => serde_json::to_string(&self.data).unwrap(),
            Output::TOML => toml::to_string(&self.data).unwrap(),
        }
    }
    fn apply_serial(&mut self, data: &str) {
        if let Ok(collurgy) = toml::from_str(data) {
            self.data = collurgy
        } else if let Ok(collurgy) = serde_json::from_str(data) {
            self.data = collurgy
        }
    }
    // }}}
}

impl App for CollurgyUI {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // {{{
        // DnD
        ctx.input(|input| {
            for f in &input.raw.dropped_files {
                if let Some(bytes) = &f.bytes {
                    if let Ok(s) = std::str::from_utf8(bytes) {
                        self.apply_serial(s)
                    }
                } else if let Some(path) = &f.path {
                    if let Ok(s) = read_to_string(path) {
                        self.apply_serial(&s)
                    }
                }
            }
        });
        let s = self.scale;
        let colors: [Color32; 16] = self.data.compute().map(|c| {
            let c = srgb_to_irgb(c);
            Color32::from_rgb(c[0], c[1], c[2])
        });
        SidePanel::right("ExportPan")
            .min_width(200.0)
            .show(ctx, |ui| {
                // EXPORTER HEADER {{{
                ui.horizontal(|ui| {
                    ui.menu_button(self.output.to_string(), |ui| {
                        let mut vals: Vec<String> = self.exporters.keys().cloned().collect();
                        vals.sort();
                        for exp in vals.into_iter() {
                            if ui.button(format!("Export/{}", &exp)).clicked() {
                                self.output = Output::Exporter(exp);
                                ui.close_menu();
                            }
                        }
                        if ui.button("Save/JSON").clicked() {
                            self.output = Output::JSON;
                            ui.close_menu();
                        }
                        if ui.button("Save/TOML").clicked() {
                            self.output = Output::TOML;
                            ui.close_menu();
                        }
                    });
                    if ui.button("Copy").clicked() {
                        ui.output_mut(|o| {
                            o.copied_text = self.process_output();
                        });
                    }
                    if ui.button("Save").clicked() {
                        let mut dialog = FileDialog::new();
                        match &self.output {
                            Output::Exporter(s) => {
                                let exp = &self.exporters[s];
                                if let Some(p) = &exp.path {
                                    if let Some(name) = p.file_name() {
                                        dialog = dialog.set_file_name(name.to_string_lossy())
                                    }
                                    if let Some(dir) = p.parent() {
                                        dialog = dialog.set_directory(dir)
                                    }
                                } else {
                                    dialog = dialog.set_file_name(&exp.name)
                                }
                            }
                            Output::TOML => dialog = dialog.set_file_name("collurgy.toml"),
                            Output::JSON => dialog = dialog.set_file_name("collurgy.json"),
                        }
                        // on Wayland this has like a 75% chance of making egui go poof
                        if let Some(file) = dialog.save_file() {
                            let _ = fs::write(file, self.process_output());
                        }
                    }
                    if ui.button("Load").clicked() {
                        let dialog = FileDialog::new()
                            .set_file_name("collurgy.toml")
                            .add_filter("Serialized Collurgy", &["toml", "json"]);
                        if let Some(path) = dialog.pick_file() {
                            if let Ok(s) = read_to_string(path) {
                                self.apply_serial(&s)
                            }
                        }
                    }
                });
                // EXPORTER HEADER }}}
                // EXPORTER {{{
                ScrollArea::both().show(ui, |ui| {
                    if let Output::Exporter(e) = &self.output {
                        if let Some(extras) = self.data.extras.get_mut(&self.exporters[e].name) {
                            let mut sorted: Vec<(&String, &mut usize)> =
                                extras.iter_mut().collect();
                            sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
                            for (id, n) in sorted.into_iter() {
                                if *n < 16 {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(id)
                                                .background_color(if *n != 0 {
                                                    colors[0]
                                                } else {
                                                    colors[15]
                                                })
                                                .color(colors[*n]),
                                        );
                                        ui.add(DragValue::new(n).clamp_range(0..=15));
                                    });
                                }
                            }
                            if ui.button("Reset All").clicked() {
                                if let Some(new_extras) = &self.exporters[e].extras {
                                    *extras = new_extras.clone()
                                }
                            }
                        }
                    }
                    // sneaky immutable textedit hack?
                    // ui.code_editor(&mut self.output().as_str());
                    // textedit always wraps???
                    ui.add(Label::new(self.process_output()).wrap(false))
                });
                // EXPORTER }}}
            });
        let fill = colcon::str2space("oklab 0.5 0 0", Space::SRGB).unwrap();
        CentralPanel::default()
            .frame(Frame::none().fill(Rgba::from_rgb(fill[0], fill[1], fill[2]).into()))
            .show(&ctx, |ui| {
                // HEADER {{{
                ui.horizontal(|ui| {
                    ui.add_sized(
                        (150.0, 20.0),
                        ColorScale::new(
                            &mut self.scale,
                            0.5..=3.0,
                            0.1,
                            format!("UI SCALE {:.1}", s),
                            colors[self.data.accent],
                            colors[0],
                            15.0,
                        ),
                    );
                    ui.add(egui::TextEdit::singleline(&mut self.data.name).desired_width(100.0));
                    ui.menu_button(format!("Model: {:?}", self.data.model), |ui| {
                        for space in [Space::HSV].iter().chain(Space::UCS_POLAR) {
                            if ui.button(format!("{:?}", space)).clicked() {
                                self.data.model = *space
                            }
                        }
                    });
                    let high2023 = self.data.high2023;
                    ui.add_sized(
                        (150.0, 20.0),
                        ColorScale::new(
                            &mut self.data.high2023,
                            -1.0..=2.0,
                            0.1,
                            format!("HIGH 2023 COMP {:.1}", high2023),
                            colors[self.data.accent],
                            colors[0],
                            15.0,
                        ),
                    );
                    Frame::none().fill(colors[0]).show(ui, |ui| {
                        ui.add_sized(
                            (300.0, 20.0),
                            Label::new(
                                RichText::new("Collurgy Theme Creator 0.1.0")
                                    .size(15.0)
                                    .color(colors[self.data.accent]),
                            ),
                        )
                    });
                });
                // HEADER }}}
                ScrollArea::both().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = (4.0 * s, 4.0 * s).into();
                    // LCH PICKERS {{{
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = (4.0 * s, 1.0 * s).into();
                        ui.add(LCH::new(
                            &mut self.data.foreground,
                            "Foreground",
                            colors[0],
                            14.0 * s,
                            s * 2.0,
                            self.data.model,
                            self.data.high2023,
                            false,
                        ));
                        ui.add(LCH::new(
                            &mut self.data.background,
                            "Background",
                            colors[15],
                            14.0 * s,
                            s * 2.0,
                            self.data.model,
                            self.data.high2023,
                            false,
                        ));
                        ui.add(LCH::new(
                            &mut self.data.spectrum,
                            "Spectrum",
                            colors[0],
                            14.0 * s,
                            s * 2.0,
                            self.data.model,
                            self.data.high2023,
                            true,
                        ));
                        ui.add(LCH::new(
                            &mut self.data.spectrum_bright,
                            "Spectrum Bright",
                            colors[0],
                            14.0 * s,
                            s * 2.0,
                            self.data.model,
                            self.data.high2023,
                            true,
                        ));
                    });
                    // LCH PICKERS }}}
                    // COLOR BUTTONS {{{
                    Grid::new("color_buttons")
                        .spacing((4.0 * s, 4.0 * s))
                        .show(ui, |ui| {
                            for n in 0..16 {
                                if ui
                                    .add_sized(
                                        (75.0 * s, 35.0 * s),
                                        ColorButton::new(
                                            format!("Color {}", n),
                                            colors[n],
                                            if n == 0 { colors[15] } else { colors[0] },
                                            15.0 * s,
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.data.accent = n
                                };
                                if n == 7 {
                                    ui.end_row()
                                }
                            }
                        });
                    // COLOR BUTTONS }}}
                    // LOREM IPSUM {{{
                    for (fg, bg) in [
                        (colors[15], colors[0]),
                        (colors[7], colors[0]),
                        (colors[15], colors[8]),
                        (colors[7], colors[8]),
                    ] {
                        Frame::none().fill(bg).inner_margin(5.0 * s).show(ui, |ui| {
                            ui.label(RichText::from(LI).color(fg).size(10.0 * s))
                        });
                    }
                    // }}}
                });
            });
    } // }}}
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

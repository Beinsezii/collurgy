use std::{env, ops::RangeInclusive};

use colcon::{convert_space, srgb_to_irgb, Space};

use eframe::{
    egui::{self, CentralPanel, Context, Frame, Grid, Label, RichText, Sense, Ui, Widget},
    emath::Align2,
    epaint::{Color32, Rounding, Stroke},
    App, CreationContext,
};

use super::Collurgy;

fn scale_factor() -> f32 {
    if let Ok(val) = env::var("GDK_DPI_SCALE") {
        val.parse::<f32>().expect("Bad GDK_DPI_SCALE value")
    } else if let Ok(val) = env::var("GDK_SCALE") {
        val.parse::<f32>().expect("Bad GDK_SCALE value")
    } else {
        1.0
    }
}

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
                .rect(response.rect, Rounding::none(), self.color, Stroke::NONE);
            ui.painter().rect(
                response.rect.shrink2(
                    (
                        response.rect.width() * 0.025,
                        response.rect.height() * 0.025,
                    )
                        .into(),
                ),
                Rounding::none(),
                self.fill,
                Stroke::NONE,
            );
        } else {
            ui.painter()
                .rect(response.rect, Rounding::none(), self.fill, Stroke::NONE);
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
            .rect(response.rect, Rounding::none(), self.color, Stroke::NONE);

        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            &self.text,
            eframe::epaint::FontId::proportional(self.font_size),
            self.fill,
        );

        ui.set_clip_rect(r2);

        ui.painter()
            .rect(response.rect, Rounding::none(), self.fill, Stroke::NONE);

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

fn lch_adjuster(
    ui: &mut Ui,
    lch: &mut [f32; 3],
    text: impl ToString,
    fill: Color32,
    font_size: f32,
) {
    ui.vertical(|ui| {
        let mut size = ui.available_size();
        size.y /= 4.0;
        let mut color = *lch;
        convert_space(Space::LCH, Space::SRGB, &mut color);
        let color = srgb_to_irgb(color);
        let color = Color32::from_rgb(color[0], color[1], color[2]);
        let vals = *lch;
        Frame::none().fill(fill).show(ui, |ui| {
            ui.add_sized(
                size,
                Label::new(RichText::new(text.to_string()).size(font_size).color(color)),
            )
        });
        ui.add_sized(
            size,
            ColorScale::new(
                &mut lch[0],
                0.0..=100.0,
                1.0,
                format!("L: {:.0}", vals[0]),
                color,
                fill,
                font_size,
            ),
        );
        ui.add_sized(
            size,
            ColorScale::new(
                &mut lch[1],
                0.0..=100.0,
                1.0,
                format!("C: {:.0}", vals[1]),
                color,
                fill,
                font_size,
            ),
        );
        ui.add_sized(
            size,
            ColorScale::new(
                &mut lch[2],
                0.0..=360.0,
                5.0,
                format!("H: {:.0}", vals[2]),
                color,
                fill,
                font_size,
            ),
        );
    });
}

pub struct CollurgyUI {
    data: Collurgy,
    scale: f32,
}

impl CollurgyUI {
    pub fn new(_cc: &CreationContext, data: Collurgy) -> Self {
        Self {
            data,
            scale: scale_factor(),
        }
    }
}

impl App for CollurgyUI {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let s = self.scale;
        let colors: [Color32; 16] = self.data.compute().map(|c| {
            let c = srgb_to_irgb(c);
            Color32::from_rgb(c[0], c[1], c[2])
        });
        CentralPanel::default()
            .frame(Frame::none().fill(colors[8]))
            .show(&ctx, |ui| {
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
                ui.spacing_mut().item_spacing = (4.0 * s, 4.0 * s).into();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = (4.0 * s, 1.0 * s).into();
                    let size = (150.0 * s, 100.0 * s).into();
                    let font = 15.0 * s;
                    ui.allocate_ui(size, |ui| {
                        lch_adjuster(ui, &mut self.data.foreground, "Foreground", colors[0], font)
                    });
                    ui.allocate_ui(size, |ui| {
                        lch_adjuster(
                            ui,
                            &mut self.data.background,
                            "Background",
                            colors[15],
                            font,
                        )
                    });
                    ui.allocate_ui(size, |ui| {
                        lch_adjuster(ui, &mut self.data.spectrum, "Spectrum", colors[0], font)
                    });
                    ui.allocate_ui(size, |ui| {
                        lch_adjuster(
                            ui,
                            &mut self.data.spectrum_bright,
                            "Bright Spectrum",
                            colors[0],
                            font,
                        )
                    });
                });
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
                    })
            });
    }
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

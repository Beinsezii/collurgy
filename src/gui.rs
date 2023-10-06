use std::{env, ops::RangeInclusive};

use colcon::{convert_space, srgb_to_irgb, Space};

use eframe::{
    egui::{self, CentralPanel, Context, Frame, Grid, Label, RichText, Sense, Ui, Widget, TextureOptions},
    emath::Align2,
    epaint::{Color32, Rounding, Stroke, Rect, ColorImage, Rgba},
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

struct LCHAdj<'a> {
    value: &'a mut [f32; 3],
    text: String,
    fill: Color32,
    font_size: f32,
    scale: f32,
}

impl<'a> LCHAdj<'a> {
    pub fn new(
        value: &'a mut [f32; 3],
        text: impl ToString,
        fill: Color32,
        font_size: f32,
        scale: f32,
    ) -> Self {
        Self {
            value,
            text: text.to_string(),
            fill,
            font_size,
            scale,
        }
    }
}

impl<'a> Widget for LCHAdj<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (chrect, chresponse) = ui.allocate_at_least((72.0 * self.scale, 100.0 * self.scale).into(), Sense::click_and_drag());
        if chresponse.dragged(){
            if let Some(pos) = chresponse.interact_pointer_pos() {
                if chrect.contains(pos) {
                    let (x, y) = (pos - chrect.left_top()).into();
                    self.value[1] = 100.0 - (y / self.scale).round();
                    self.value[2] = (x / self.scale).round() * 5.0;
                    println!("{:?}", self.value);
                }
            }
        }
        let chpaint = ui.painter_at(chrect);
        let mut pixels = [self.value[0]; 3 * 72 * 101];
        for c in 0..=100 {
            for h in 0..72 {
                let index = (h + c * 72) * 3;
                let p: &mut [f32; 3] = pixels.get_mut(index..(index+3)).unwrap().try_into().unwrap();
                p[1] = 100.0 - c as f32;
                p[2] = h as f32 * 5.0;
                convert_space(Space::LCH, Space::LRGB, p);
            }
        }
        let img = ColorImage{
            size: [72, 101],
            pixels: pixels.chunks_exact(3).map(|p| Rgba::from_rgba_unmultiplied(p[0], p[1], p[2], 1.0).into()).collect::<Vec<Color32>>(),
        };
        let texture = ui.ctx().load_texture(self.text, img, TextureOptions::NEAREST);
        chpaint.image(texture.id(), chrect, Rect::from_min_max((0.0, 0.0).into(), (1.0, 1.0).into()), Color32::WHITE);
        let chpos = chrect.left_top() + (self.value[2] / 5.0 * self.scale, (100.0 - self.value[1]) * self.scale).into();

        for (x, y) in [(0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
            chpaint.line_segment([chpos + (x * self.scale, y * self.scale).into(), chpos + (x * 4.0 * self.scale, y * 4.0 * self.scale).into()], Stroke {
                color: self.fill,
                width: 1.0 * self.scale,
            });
        }

        chresponse
    }
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
                    ui.add(LCHAdj::new(&mut self.data.foreground, "Foreground", colors[0], 15.0, s * 2.0));
                    ui.add(LCHAdj::new(&mut self.data.background, "Background", colors[15], 15.0, s * 2.0));
                    ui.add(LCHAdj::new(&mut self.data.spectrum, "Spectrum", colors[0], 15.0, s * 2.0));
                    ui.add(LCHAdj::new(&mut self.data.spectrum_bright, "Spectrum Bright", colors[0], 15.0, s * 2.0));
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

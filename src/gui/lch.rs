use crate::apply_space;
use colcon::Space;
use eframe::{
    egui::{self, Label, RichText, Sense, TextureOptions, Widget},
    epaint::{Color32, ColorImage, Rect, Rgba, Stroke},
};

pub struct LCH<'a> {
    value: &'a mut [f32; 3],
    text: String,
    fill: Color32,
    font_size: f32,
    scale: f32,
    space: Space,
    high2023: f32,
    spectrum: bool,
    clip: bool,
}

fn clip(pixels: &mut [[f32; 3]]) {
    let fill: [f32; 3] = colcon::str2space("oklab 50% 0 0", Space::SRGB).unwrap();
    pixels.iter_mut().for_each(|p| {
        if p.iter().any(|c| *c < 0.0 || *c > 1.0) {
            *p = fill
        }
    })
}

impl<'a> LCH<'a> {
    pub fn new(
        value: &'a mut [f32; 3],
        text: impl ToString,
        fill: Color32,
        font_size: f32,
        scale: f32,
        space: Space,
        high2023: f32,
        spectrum: bool,
        clip: bool,
    ) -> Self {
        Self {
            value,
            text: text.to_string(),
            fill,
            font_size,
            scale,
            space,
            high2023,
            spectrum,
            clip,
        }
    }
}

impl<'a> Widget for LCH<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            let mut fg = [*self.value];
            apply_space(self.space, &mut fg, Space::LRGB, self.high2023);
            let fg = fg[0];
            let fg: Color32 = Rgba::from_rgb(fg[0], fg[1], fg[2]).into();
            ui.add(
                Label::new(
                    RichText::new(format!(
                        "{} {:.0} {:.0} {:.0}",
                        self.text, self.value[0], self.value[1], self.value[2]
                    ))
                    .size(self.font_size)
                    .background_color(self.fill)
                    .color(fg),
                )
                .wrap(false),
            );
            let texres = ui
                .horizontal(|ui| {
                    let (chrect, chresponse) = ui.allocate_at_least(
                        (72.0 * self.scale, 101.0 * self.scale).into(),
                        Sense::click_and_drag(),
                    );
                    let (lrect, lresponse) = ui.allocate_at_least(
                        (10.0 * self.scale, 101.0 * self.scale).into(),
                        Sense::click_and_drag(),
                    );
                    if chresponse.dragged() {
                        if let Some(pos) = chresponse.interact_pointer_pos() {
                            if chrect.contains(pos) {
                                let (x, y) = (pos - chrect.left_top()).into();
                                self.value[1] = 100.0 - (y / self.scale - 0.5).round().max(0.0);
                                self.value[2] =
                                    (x / self.scale - 0.5).round().clamp(0.0, 71.0).abs() * 5.0;
                            }
                        }
                    }
                    if lresponse.dragged() {
                        if let Some(pos) = lresponse.interact_pointer_pos() {
                            if lrect.contains(pos) {
                                let y = (pos - lrect.left_top()).y;
                                self.value[0] = 100.0 - (y / self.scale - 0.5).round().max(0.0);
                            }
                        }
                    }
                    // CH Square
                    let chpaint = ui.painter_at(chrect);

                    let mut pixels: Vec<[f32; 3]> = (0..=100)
                        .map(|c| {
                            (0..72)
                                .map(|h| [self.value[0], (100 - c) as f32, h as f32 * 5.0])
                                .collect::<Vec<[f32; 3]>>()
                        })
                        .reduce(|mut acc, e| {
                            acc.extend_from_slice(&e);
                            acc
                        })
                        .unwrap();

                    apply_space(self.space, &mut pixels, Space::LRGB, self.high2023);
                    if self.clip {
                        clip(&mut pixels)
                    }

                    let chimg = ColorImage {
                        size: [72, 101],
                        pixels: pixels
                            .into_iter()
                            .map(|p| Rgba::from_rgb(p[0], p[1], p[2]).into())
                            .collect::<Vec<Color32>>(),
                    };
                    let chtexture = ui.ctx().load_texture(
                        format!("{} CH", self.text),
                        chimg,
                        TextureOptions::NEAREST,
                    );
                    chpaint.image(
                        chtexture.id(),
                        chrect,
                        Rect::from_min_max((0.0, 0.0).into(), (1.0, 1.0).into()),
                        Color32::WHITE,
                    );
                    let chpos = chrect.left_top()
                        + (
                            (self.value[2] / 5.0 + 0.5) * self.scale,
                            (100.0 - self.value[1] + 0.5) * self.scale,
                        )
                            .into();

                    // crosshair
                    for (x, y) in [(0.0, 0.5), (0.0, -0.5), (0.5, 0.0), (-0.5, 0.0)] {
                        chpaint.line_segment(
                            [
                                chpos + (x * self.scale, y * self.scale).into(),
                                chpos + (x * 5.0 * self.scale, y * 5.0 * self.scale).into(),
                            ],
                            Stroke {
                                color: self.fill,
                                width: 0.5 * self.scale,
                            },
                        );
                    }
                    // other ticks
                    if self.spectrum {
                        for y in [0.5, -0.5] {
                            for n in 1..6 {
                                let pos = egui::Pos2 {
                                    x: (chpos.x - chrect.left() + n as f32 * chrect.width() / 6.0)
                                        .rem_euclid(chrect.width())
                                        + chrect.left(),
                                    y: chpos.y,
                                };
                                chpaint.line_segment(
                                    [
                                        pos + (0.0, y * self.scale).into(),
                                        pos + (0.0, y * 3.0 * self.scale).into(),
                                    ],
                                    Stroke {
                                        color: self.fill,
                                        width: 0.5 * self.scale,
                                    },
                                );
                            }
                        }
                    }

                    // L slider
                    let lpaint = ui.painter_at(lrect);

                    let mut pixels: Vec<[f32; 3]> = (0..=100)
                        .map(|l| [(100 - l) as f32, self.value[1], self.value[2]])
                        .collect::<Vec<[f32; 3]>>();

                    apply_space(self.space, &mut pixels, Space::LRGB, self.high2023);
                    if self.clip {
                        clip(&mut pixels)
                    }

                    let limg = ColorImage {
                        size: [1, 101],
                        pixels: pixels
                            .into_iter()
                            .map(|p| Rgba::from_rgb(p[0], p[1], p[2]).into())
                            .collect::<Vec<Color32>>(),
                    };
                    let ltexture = ui.ctx().load_texture(
                        format!("{} L", self.text),
                        limg,
                        TextureOptions::NEAREST,
                    );
                    lpaint.image(
                        ltexture.id(),
                        lrect,
                        Rect::from_min_max((0.0, 0.0).into(), (1.0, 1.0).into()),
                        Color32::WHITE,
                    );
                    let lpos = lrect.center_top()
                        + (0.0, ((100.0 - self.value[0] + 0.5) * self.scale)).into();
                    // crosshair
                    for (x, y) in [(0.0, 0.5), (0.0, -0.5), (0.5, 0.0), (-0.5, 0.0)] {
                        lpaint.line_segment(
                            [
                                lpos + (x * self.scale, y * self.scale).into(),
                                lpos + (x * 7.0 * self.scale, y * 3.0 * self.scale).into(),
                            ],
                            Stroke {
                                color: self.fill,
                                width: 0.5 * self.scale,
                            },
                        );
                    }
                    chresponse.union(lresponse)
                })
                .response;

            // let mut hex = [*self.value];
            // apply_space(self.space, &mut hex, Space::SRGB, self.high2023);
            // let hex = colcon::irgb_to_hex(colcon::srgb_to_irgb(hex[0]));
            // let mut buff = hex.clone();
            // egui::Frame::none().fill(self.fill).show(ui, |ui| {
            //     ui.add_sized(
            //         (self.font_size * 5.0, self.font_size),
            //         egui::TextEdit::singleline(&mut buff)
            //             .font(egui::FontId::monospace(self.font_size))
            //             .text_color(fg)
            //             .frame(false),
            //     )
            // });

            texres
        })
        .response
    }
}

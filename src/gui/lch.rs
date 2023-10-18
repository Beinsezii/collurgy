use colcon::{convert_space, Space, hk_comp_2023};
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
}

impl<'a> LCH<'a> {
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

impl<'a> Widget for LCH<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.vertical(|ui| {
            let mut fg = *self.value;
            hk_comp_2023(&mut fg);
            convert_space(Space::LCH, Space::LRGB, &mut fg);
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
                        (72.0 * self.scale, 100.0 * self.scale).into(),
                        Sense::click_and_drag(),
                    );
                    let (lrect, lresponse) = ui.allocate_at_least(
                        (10.0 * self.scale, 100.0 * self.scale).into(),
                        Sense::click_and_drag(),
                    );
                    if chresponse.dragged() {
                        if let Some(pos) = chresponse.interact_pointer_pos() {
                            if chrect.contains(pos) {
                                let (x, y) = (pos - chrect.left_top()).into();
                                self.value[1] = 100.0 - (y / self.scale).round();
                                self.value[2] = (x / self.scale).round() * 5.0;
                            }
                        }
                    }
                    if lresponse.dragged() {
                        if let Some(pos) = lresponse.interact_pointer_pos() {
                            if lrect.contains(pos) {
                                let y = (pos - lrect.left_top()).y;
                                self.value[0] = 100.0 - (y / self.scale).round();
                            }
                        }
                    }
                    // CH Square
                    let chpaint = ui.painter_at(chrect);
                    let chimg = ColorImage {
                        size: [72, 101],
                        pixels: (0..=100)
                            .map(|c| {
                                (0..72)
                                    .map(|h| {
                                        let mut p =
                                            [self.value[0], (100 - c) as f32, h as f32 * 5.0];
                                        hk_comp_2023(&mut p);
                                        convert_space(Space::LCH, Space::LRGB, &mut p);
                                        Rgba::from_rgb(p[0], p[1], p[2]).into()
                                    })
                                    .collect::<Vec<Color32>>()
                            })
                            .reduce(|mut acc, mut e| {
                                acc.append(&mut e);
                                acc
                            })
                            .unwrap(),
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
                            self.value[2] / 5.0 * self.scale,
                            (100.0 - self.value[1]) * self.scale,
                        )
                            .into();

                    for (x, y) in [(0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
                        chpaint.line_segment(
                            [
                                chpos + (x * self.scale, y * self.scale).into(),
                                chpos + (x * 4.0 * self.scale, y * 4.0 * self.scale).into(),
                            ],
                            Stroke {
                                color: self.fill,
                                width: 1.0 * self.scale,
                            },
                        );
                    }

                    // L slider
                    let lpaint = ui.painter_at(lrect);

                    let limg = ColorImage {
                        size: [1, 101],
                        pixels: (0..=100)
                            .map(|n| {
                                let mut p = [(100 - n) as f32, self.value[1], self.value[2]];
                                hk_comp_2023(&mut p);
                                convert_space(Space::LCH, Space::LRGB, &mut p);
                                Rgba::from_rgb(p[0], p[1], p[2]).into()
                            })
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
                    let lpos =
                        lrect.center_top() + (0.0, ((100.0 - self.value[0]) * self.scale)).into();
                    for (x, y) in [(0.0, 1.0), (0.0, -1.0), (1.0, 0.0), (-1.0, 0.0)] {
                        lpaint.line_segment(
                            [
                                lpos + (x * self.scale, y * self.scale).into(),
                                lpos + (x * 4.0 * self.scale, y * 4.0 * self.scale).into(),
                            ],
                            Stroke {
                                color: self.fill,
                                width: 1.0 * self.scale,
                            },
                        );
                    }
                    chresponse.union(lresponse)
                })
                .response;

            // let mut hex = *self.value;
            // convert_space(Space::LCH, Space::SRGB, &mut hex);
            // let hex = irgb_to_hex(srgb_to_irgb(hex));
            // let mut buff = hex.clone();
            // Frame::none().fill(self.fill).show(ui, |ui| {
            //     ui.add(
            //         TextEdit::singleline(&mut buff)
            //             .font(FontId::monospace(self.font_size))
            //             .text_color(fg)
            //             .min_size((self.font_size * 2.0, self.font_size).into())
            //             .frame(false),
            //     )
            // });

            texres
        })
        .response
    }
}

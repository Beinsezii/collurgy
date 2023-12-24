use std::{collections::HashMap, ffi::OsStr, fs::read_to_string, path::PathBuf};

use colcon::{convert_space_chunked, irgb_to_hex, srgb_to_irgb, Space, hk_high2023_comp};
use serde::{Deserialize, Serialize};

mod gui;
use gui::CollurgyUI;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum Model {
    HSV,
    CIELCH,
    CIELCH2023,
    OKLCH,
    JZCZHZ,
}

impl Model {
    fn apply(&self, colors: &mut [[f32; 3]], to: colcon::Space) {
        let from = match self {
            Model::HSV => {colors.iter_mut().for_each(|col| *col = [col[2] / 360.0, col[1] / 100.0, col[0] / 100.0]); Space::HSV},
            Model::CIELCH => Space::LCH,
            Model::CIELCH2023 => {colors.iter_mut().for_each(|col| hk_high2023_comp(col)); Space::LCH},
            Model::OKLCH => {colors.iter_mut().for_each(|col| {col[0] /= 100.0; col[1] /= 400.0;}); Space::OKLCH},
            Model::JZCZHZ => {colors.iter_mut().for_each(|col| {col[0] /= 5650.0; col[1] /= 5650.0;}); Space::JZCZHZ}
        };
        convert_space_chunked(from, to, colors);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Collurgy {
    model: Model,
    /// LCH
    foreground: [f32; 3],
    /// LCH
    background: [f32; 3],
    /// LCH
    spectrum: [f32; 3],
    /// LCH
    spectrum_bright: [f32; 3],
    /// Which # should be accent
    accent: usize,
}

impl Default for Collurgy {
    fn default() -> Self {
        Self {
            model: Model::CIELCH,
            foreground: [100.0, 0.0, 0.0],
            background: [0.0; 3],
            spectrum: [50.0, 50.0, 30.0],
            spectrum_bright: [70.0, 50.0, 30.0],
            accent: 11, // Bright Yellow
        }
    }
}

impl Collurgy {
    /// returns all 16 colors in SRGB
    fn compute(&self) -> [[f32; 3]; 16] {
        let mut result = [[0.0; 3]; 16];

        result[0] = self.background;
        result[8] = self.background;
        // 1/3 distance to foreground
        result[8]
            .as_mut_slice()
            .iter_mut()
            .zip(self.foreground.as_slice().iter())
            .for_each(|(a, b)| *a = (*a * 2.0 + *b) / 3.0);

        result[7] = self.foreground;
        result[15] = self.foreground;
        // 1/3 distance to background
        result[7]
            .as_mut_slice()
            .iter_mut()
            .zip(self.background.as_slice().iter())
            .for_each(|(a, b)| *a = (*a * 2.0 + *b) / 3.0);

        let mut rots = (0..6).into_iter().map(|n| {
            [
                self.spectrum[0],
                self.spectrum[1],
                60.0 * (n as f32) + self.spectrum[2],
            ]
        });

        result[1] = rots.next().unwrap(); // Red
        result[3] = rots.next().unwrap(); // Yellow
        result[2] = rots.next().unwrap(); // Green
        result[6] = rots.next().unwrap(); // Cyan
        result[4] = rots.next().unwrap(); // Blue
        result[5] = rots.next().unwrap(); // Magenta

        let mut brots = (0..6).into_iter().map(|n| {
            [
                self.spectrum_bright[0],
                self.spectrum_bright[1],
                60.0 * (n as f32) + self.spectrum_bright[2],
            ]
        });

        result[9] = brots.next().unwrap(); // Red
        result[11] = brots.next().unwrap(); // Yellow
        result[10] = brots.next().unwrap(); // Green
        result[14] = brots.next().unwrap(); // Cyan
        result[12] = brots.next().unwrap(); // Blue
        result[13] = brots.next().unwrap(); // Magenta

        self.model.apply(&mut result, Space::SRGB);

        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct Exporter {
    name: String,
    formatter: String,
    path: Option<PathBuf>,
    extras: Option<HashMap<String, usize>>,
}

impl Exporter {
    fn export(&self, data: &Collurgy, extras: &Option<HashMap<String, usize>>) -> String {
        let frgb = data.compute();
        let irgb = frgb.map(|pixel| srgb_to_irgb(pixel));
        let hex = irgb.map(|pixel| irgb_to_hex(pixel));
        let mut result = self.formatter.clone();
        let mut swaps = irgb
            .iter()
            .zip(frgb.iter())
            .zip(hex.iter())
            .enumerate()
            .map(|(n, ((ip, fp), hex))| {
                vec![
                    (format!("{{R{}}}", n), ip[0].to_string()),
                    (format!("{{G{}}}", n), ip[1].to_string()),
                    (format!("{{B{}}}", n), ip[2].to_string()),
                    (format!("{{FR{}}}", n), fp[0].to_string()),
                    (format!("{{FG{}}}", n), fp[1].to_string()),
                    (format!("{{FB{}}}", n), fp[2].to_string()),
                    (format!("{{HEX{}}}", n), hex.clone()),
                ]
            })
            .reduce(|mut a, mut e| {
                a.append(&mut e);
                a
            })
            .unwrap();

        swaps.append(&mut vec![
            ("{ACCR}".to_string(), irgb[data.accent][0].to_string()),
            ("{ACCG}".to_string(), irgb[data.accent][1].to_string()),
            ("{ACCB}".to_string(), irgb[data.accent][2].to_string()),
            ("{ACCFR}".to_string(), frgb[data.accent][0].to_string()),
            ("{ACCFG}".to_string(), frgb[data.accent][1].to_string()),
            ("{ACCFB}".to_string(), frgb[data.accent][2].to_string()),
            ("{ACCHEX}".to_string(), hex[data.accent].clone()),
        ]);

        if let Some(ext) = extras.as_ref().or(self.extras.as_ref()) {
            for (id, n) in ext {
                if let (Some(iv), Some(fv), Some(hv)) = (irgb.get(*n), frgb.get(*n), hex.get(*n)) {
                    swaps.append(&mut vec![
                        (format!("{{{}R}}", id), iv[0].to_string()),
                        (format!("{{{}G}}", id), iv[1].to_string()),
                        (format!("{{{}B}}", id), iv[2].to_string()),
                        (format!("{{{}FR}}", id), fv[0].to_string()),
                        (format!("{{{}FG}}", id), fv[1].to_string()),
                        (format!("{{{}FB}}", id), fv[2].to_string()),
                        (format!("{{{}HEX}}", id), hv.to_string()),
                    ]);
                }
            }
        }

        for (a, b) in swaps {
            result = result.replace(&a, &b)
        }
        result
    }
}

fn collect_exporters(paths: Vec<PathBuf>) -> HashMap<String, Exporter> {
    let mut result = HashMap::new();
    #[cfg(feature = "builtins")]
    for builtin in [
        include_str!("../builtins/dunst.toml"),
        include_str!("../builtins/dwarf.toml"),
        include_str!("../builtins/i3.toml"),
        include_str!("../builtins/kitty.toml"),
        include_str!("../builtins/ppm.toml"),
        include_str!("../builtins/vim.toml"),
        include_str!("../builtins/xresources.toml"),
    ] {
        let exporter = toml::from_str::<Exporter>(builtin).unwrap();
        result.insert(exporter.name.clone(), exporter);
    }
    let mut found = Vec::new();
    for p in paths {
        if p.is_dir() {
            if let Ok(files) = p.read_dir() {
                for f in files.filter_map(|f| f.ok()) {
                    if f.path().extension() == Some(OsStr::new("toml")) {
                        if let Ok(s) = read_to_string(f.path()) {
                            found.push(s)
                        }
                    }
                }
            }
        } else if p.extension() == Some(OsStr::new("toml")) {
            if let Ok(s) = read_to_string(p) {
                found.push(s)
            }
        }
    }
    for s in found.iter() {
        if let Ok(exporter) = toml::from_str::<Exporter>(s.as_str()) {
            result.insert(exporter.name.clone(), exporter);
        }
    }
    result
}

fn main() {
    eframe::run_native(
        "Collurgy",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| {
            Box::new(CollurgyUI::new(
                cc,
                Collurgy::default(),
                collect_exporters(vec![PathBuf::from("./exporters/")]),
            ))
        }),
    )
    .unwrap();
}

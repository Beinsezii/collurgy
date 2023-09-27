use colcon::{convert_space, Space};
use serde::{Deserialize, Serialize};

mod gui;
use gui::CollurgyUI;

#[derive(Serialize, Deserialize)]
pub struct Collurgy {
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
            foreground: [100.0, 0.0, 0.0],
            background: [0.0; 3],
            spectrum: [35.0, 35.0, 0.0],
            spectrum_bright: [65.0, 65.0, 0.0],
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

        result
            .iter_mut()
            .for_each(|col| convert_space(Space::LCH, Space::SRGB, col));

        result
    }
}

fn main() {
    eframe::run_native(
        "Collurgy",
        eframe::NativeOptions {
            ..Default::default()
        },
        Box::new(|cc| Box::new(CollurgyUI::new(cc, Collurgy::default()))),
    )
    .unwrap();
}

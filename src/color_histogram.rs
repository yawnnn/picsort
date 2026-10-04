use std::{
    ops::{Deref, DerefMut},
    *,
};

use image::DynamicImage;
use picsort::*;

#[derive(Default)]
pub struct Histogram([f32; Self::BINS]);

impl Histogram {
    pub const BINS: usize = 32;

    /// chi-squared distance
    pub fn dist(&self, other: &Self) -> f32 {
        self.iter()
            .zip(other.iter())
            .filter(|(a, b)| *a + *b > 0.)
            .fold(0., |acc, (&a, &b)| acc + (a - b).powi(2) / (a + b))
    }
}

impl Deref for Histogram {
    type Target = [f32; Self::BINS];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Histogram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn map_color_histograms<'a>(
    imgs: impl IntoIterator<Item = &'a DynamicImage>,
) -> impl Iterator<Item = Histogram> {
    imgs.into_iter().map(|img| {
        let img = img.to_rgb8();
        let dim = img.dimensions();
        let pixel_count = dim.0 * dim.1;
        let mut histogram = Histogram::default();
        for px in img.pixels() {
            let hsv = rgb_to_hsv(px.0);
            let hue = hsv.hue.into_positive_degrees();
            let idx = map_range_from_0(hue, 360., (Histogram::BINS - 1) as f32)
                .floor();
            histogram[idx as usize] += 1.;
        }
        // normalize to pixel count
        histogram.iter_mut().for_each(|v| *v /= pixel_count as f32);
        histogram
    })
}

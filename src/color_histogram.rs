use std::{
    ops::{Deref, DerefMut},
    *,
};

use image::{DynamicImage, GenericImageView, Pixel};
use palette::{Hsv, IntoColor, Srgb};
use crate::*;

pub fn rgb_to_hsv(rgb: [u8; 3]) -> Hsv {
    Srgb::from(rgb).into_format::<f32>().into_color()
}

pub fn map_range_from_0<T: Mul<Output = T> + Div<Output = T>>(
    n: T,
    src_end: T,
    dst_end: T,
) -> T {
    (n / src_end) * dst_end
}

#[derive(Default)]
pub struct Histogram([f32; Self::BINS]);

impl Histogram {
    pub const BINS: usize = 32;

    /// Chi-squared distance
    pub fn dist(&self, other: &Self) -> f32 {
        self.iter()
            .zip(other.iter())
            .filter(|(a, b)| *a + *b > 0.)
            .fold(0., |acc, (&a, &b)| acc + (a - b).powi(2) / (a + b))
    }

    pub fn dist_normalized(&self, other: &Self) -> f32 {
        self.dist(other) / 2.
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

impl From<&DynamicImage> for Histogram {
    fn from(img: &DynamicImage) -> Self {
        let mut histogram = Histogram::default();
        for (_, _, rgba) in img.pixels() {
            let hsv = rgb_to_hsv(rgba.to_rgb().0);
            let hue = hsv.hue.into_positive_degrees();
            let idx = map_range_from_0(hue, 360., (Histogram::BINS - 1) as f32)
                .floor();
            histogram[idx as usize] += 1.;
        }
        // normalize to pixel count
        let dim = img.dimensions();
        let pixel_count = dim.0 * dim.1;
        histogram.iter_mut().for_each(|v| *v /= pixel_count as f32);
        histogram
    }
}

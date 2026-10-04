use image::{DynamicImage, GenericImageView, imageops::FilterType};

pub struct DHash(u64);

impl DHash {
    const WIDTH: u32 = 9;
    const HEIGHT: u32 = 8;

    /// Hamming distance
    pub fn dist(&self, other: &Self) -> u8 {
        (self.0 ^ other.0).count_ones() as u8
    }

    pub fn dist_normalized(&self, other: &Self) -> f32 {
        self.dist(other) as f32 / 64.
    }
}

impl From<&DynamicImage> for DHash {
    fn from(img: &DynamicImage) -> Self {
        let img = img.grayscale().resize_exact(
            Self::WIDTH,
            Self::HEIGHT,
            FilterType::Triangle,
        );

        let mut hash = 0;

        for x in 0..(Self::WIDTH - 1) {
            for y in 0..Self::HEIGHT {
                let bit = x * Self::HEIGHT + y;
                let curr = img.get_pixel(x, y).0[0];
                let next = img.get_pixel(x + 1, y).0[0];
                if curr > next {
                    hash |= 1 << bit;
                }
            }
        }

        DHash(hash)
    }
}

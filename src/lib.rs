use std::{
    ops::{Div, Mul},
    *,
};

use image::{DynamicImage, ImageFormat, ImageReader, ImageResult};
use palette::{Hsv, IntoColor, Srgb};

// some extensions (eg. `riff`) will error out, even though the format can be guessed by the contents
pub fn open_img_with_guessed_fmt(
    p: &path::Path,
) -> ImageResult<(DynamicImage, ImageFormat)> {
    let reader = ImageReader::new(io::BufReader::new(fs::File::open(p)?))
        .with_guessed_format()?;
    let fmt = reader.format().unwrap();
    let img = reader.decode()?;
    Ok((img, fmt))
}

pub fn rgb_to_hsv(rgb: [u8; 3]) -> Hsv {
    Srgb::from(rgb).into_format::<f32>().into_color()
}

// pub fn hsv_to_rgb(hsv: Hsv) -> [u8; 3] {
//     let srgb: Srgb<f32> = hsv.into_color();
//     srgb.into_format::<u8>().into()
// }

pub fn map_range_from_0<T: Mul<Output = T> + Div<Output = T>>(
    n: T,
    src_end: T,
    dst_end: T,
) -> T {
    (n / src_end) * dst_end
}
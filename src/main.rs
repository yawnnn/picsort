use std::{
    cmp::Ordering,
    io::Write,
    ops::{Div, Mul},
    *,
};

use image::{DynamicImage, ImageFormat, ImageReader, ImageResult};
use palette::{Hsv, IntoColor, Srgb};

const DIR_RAW_PICS: &str = "raw";
const DIR_PICS: &str = "pics";
const DIR_OUT: &str = "out";
const MAX_WIDTH: usize = 32;
const MAX_HEIGHT: usize = 32;

fn list_pics(folder: &str) -> Vec<path::PathBuf> {
    fs::read_dir(folder)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()).filter(|p| p.is_file()))
        .collect()
}

/// Natural compare - splits strings into text/number chunks
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num: String = a_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let b_num: String = b_chars
                        .by_ref()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();

                    match a_num.len().cmp(&b_num.len()) {
                        Ordering::Equal => match a_num.cmp(&b_num) {
                            Ordering::Equal => continue,
                            ord => return ord,
                        },
                        ord => return ord,
                    }
                } else {
                    let ac_lower = ac.to_ascii_lowercase();
                    let bc_lower = bc.to_ascii_lowercase();
                    match ac_lower.cmp(&bc_lower) {
                        Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                        }
                        ord => return ord,
                    }
                }
            }
        }
    }
}

fn list_pics_natural_sort(folder: &str) -> Vec<path::PathBuf> {
    let mut pics = list_pics(folder);
    pics.sort_by(|a, b| {
        natural_cmp(
            &a.file_stem().unwrap().to_string_lossy(),
            &b.file_stem().unwrap().to_string_lossy(),
        )
    });
    pics
}

fn num_digits(n: usize) -> usize {
    n.ilog10() as usize + 1
}

#[allow(unused)]
fn initial_rename() -> io::Result<()> {
    let mut log = fs::File::create(
        path::PathBuf::from(DIR_RAW_PICS)
            .join("notes")
            .join("initial_rename.log"),
    )
    .unwrap();
    log.write_all(b"[\n").unwrap();
    let pics = list_pics_natural_sort(DIR_RAW_PICS);
    let max_digits = num_digits(pics.len());
    for (i, p) in pics.into_iter().enumerate() {
        let new = p
            .with_file_name(format!("{i:0>max_digits$}"))
            .with_extension(p.extension().unwrap());
        writeln!(log, "({p:?}, {new:?}),").unwrap();
        fs::rename(&p, new).unwrap();
    }
    log.write_all(b"\n]").unwrap();
    Ok(())
}

// some extensions (eg. `riff`) will error out, even though the format can be guessed by the contents
fn open_img_with_guessed_fmt(
    p: &path::Path,
) -> ImageResult<(DynamicImage, ImageFormat)> {
    let reader = ImageReader::new(io::BufReader::new(fs::File::open(p)?))
        .with_guessed_format()?;
    let fmt = reader.format().unwrap();
    let img = reader.decode()?;
    Ok((img, fmt))
}

#[allow(unused)]
fn initial_downsample() {
    if fs::exists(DIR_PICS).unwrap() {
        let exit = process::Command::new("rm")
            .arg("-rf")
            .arg(DIR_PICS)
            .status()
            .unwrap();
        assert!(exit.success());
    }
    fs::create_dir_all(DIR_PICS).unwrap();
    for p in list_pics(DIR_RAW_PICS) {
        let (img, fmt) = open_img_with_guessed_fmt(&p).unwrap();
        let img = img.resize(
            MAX_WIDTH as u32,
            MAX_HEIGHT as u32,
            image::imageops::FilterType::Triangle,
        );
        img.save_with_format(
            path::PathBuf::from(DIR_PICS).join(p.file_name().unwrap()),
            fmt,
        )
        .unwrap();
    }
}

fn rgb_to_hsv(rgb: [u8; 3]) -> Hsv {
    Srgb::from(rgb).into_format::<f32>().into_color()
}

// fn hsv_to_rgb(hsv: Hsv) -> [u8; 3] {
//     let srgb: Srgb<f32> = hsv.into_color();
//     srgb.into_format::<u8>().into()
// }

fn map_range_from_0<T: Mul<Output = T> + Div<Output = T>>(
    n: T,
    src_end: T,
    dst_end: T,
) -> T {
    (n / src_end) * dst_end
}

/// Calculate chi-squared distance
fn histogram_dist(h1: &[f32], h2: &[f32]) -> f32 {
    h1.iter()
        .zip(h2.iter())
        .filter(|(a, b)| *a + *b > 0.)
        .fold(0., |acc, (&a, &b)| acc + (a - b).powi(2) / (a + b))
}

fn main() {
    let pics = list_pics(DIR_PICS);

    // create histograms
    const BINS: usize = 32;
    let mut histograms: Vec<_> = pics
        .iter()
        .map(|p| {
            let (img, _) = open_img_with_guessed_fmt(p).unwrap();
            let img = img.to_rgb8();
            let dim = img.dimensions();
            let pixel_count = dim.0 * dim.1;
            let mut histogram: Box<[f32; BINS]> = Box::default();
            for px in img.pixels() {
                let hsv = rgb_to_hsv(px.0);
                let hue = hsv.hue.into_positive_degrees();
                let idx =
                    map_range_from_0(hue, 360., (BINS - 1) as f32).floor();
                histogram[idx as usize] += 1.;
            }
            // normalize to pixel count
            histogram.iter_mut().for_each(|v| *v /= pixel_count as f32);
            (p, histogram)
        })
        .collect();

    // sort histograms
    let mut sorted = vec![histograms.remove(0)];
    while !histograms.is_empty() {
        let last = sorted.last().unwrap().1.as_slice();
        let (idx, _) = histograms
            .iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| {
                let dist_a = histogram_dist(last, a.as_slice());
                let dist_b = histogram_dist(last, b.as_slice());
                dist_a
                    .partial_cmp(&dist_b)
                    .unwrap_or_else(|| panic!("[{dist_a}, {dist_b}]"))
            })
            .unwrap();
        sorted.push(histograms.remove(idx));
    }
    assert_eq!(pics.len(), sorted.len());

    // create reordered result
    fs::create_dir_all(DIR_OUT).unwrap();
    let max_digits = num_digits(pics.len());
    for (new_i, (p, _)) in sorted.iter().enumerate() {
        let ext = p.extension().unwrap().to_string_lossy();
        fs::copy(
            p,
            path::PathBuf::from(DIR_OUT)
                .join(format!("{new_i:0>max_digits$}.{ext}")),
        )
        .unwrap();
    }
}

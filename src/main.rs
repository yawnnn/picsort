use std::{fs, path};

use color_histogram::*;
use dhash::*;
use image::DynamicImage;
use picsort::*;

mod color_histogram;
mod dhash;

const DIR_INPUT: &str = "input";
const MAX_WIDTH: u32 = 256;
const MAX_HEIGHT: u32 = 256;

fn list_pics(folder: &str) -> Vec<path::PathBuf> {
    fs::read_dir(folder)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()).filter(|p| p.is_file()))
        .collect()
}

struct ImgHash {
    histogram: Histogram,
    dhash: DHash,
}

impl ImgHash {
    const HISTOGRAM_WEIGHT: f32 = 0.5;
    const DHASH_WEIGHT: f32 = 0.5;

    fn dist(&self, other: &Self) -> f32 {
        Self::HISTOGRAM_WEIGHT
            * self.histogram.dist_normalized(&other.histogram)
            + Self::DHASH_WEIGHT * self.dhash.dist_normalized(&other.dhash)
    }
}

impl From<&DynamicImage> for ImgHash {
    fn from(img: &image::DynamicImage) -> Self {
        Self {
            histogram: Histogram::from(img),
            dhash: DHash::from(img),
        }
    }
}

fn main() {
    let paths = list_pics(DIR_INPUT);

    let imgs: Vec<_> = paths
        .iter()
        .map(|p| {
            let img = open_img_with_guessed_fmt(p).unwrap().0;
            img.resize(
                MAX_WIDTH,
                MAX_HEIGHT,
                image::imageops::FilterType::Triangle,
            )
        })
        .collect();

    // calc hashes
    let mut hashes: Vec<_> = imgs
        .iter()
        .zip(&paths)
        .map(|(img, p)| (p, ImgHash::from(img)))
        .collect();

    // sort
    let mut sorted = vec![hashes.remove(0)];
    while !hashes.is_empty() {
        let last = &sorted.last().unwrap().1;
        let (idx, _) = hashes
            .iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| {
                last.dist(a).partial_cmp(&last.dist(b)).unwrap()
            })
            .unwrap();
        sorted.push(hashes.remove(idx));
    }

    // create output
    let max_digits = paths.len().ilog10() as usize + 1;
    for (new_i, (old, _)) in sorted.iter().enumerate() {
        let mut old_gen = u32::MAX;
        let mut old_i = usize::MAX;
        if let Some((old_gen_s, old_i_s)) = old
            .file_stem()
            .unwrap()
            .to_str()
            .and_then(|s| s.split_once('_'))
        {
            old_gen = old_gen_s.parse::<u32>().unwrap_or(old_gen);
            old_i = old_i_s.parse::<usize>().unwrap_or(old_i)
        }
        let new_gen = old_gen.wrapping_add(1);
        let new = old
            .parent()
            .unwrap()
            .join(format!("{new_gen}_{new_i:0>max_digits$}"))
            .with_extension(old.extension().unwrap_or_default());
        if old_i == new_i {
            println!("{old:?} => {new:?}");
        }
        fs::rename(old, new).unwrap();
    }
}

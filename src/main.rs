use clap::Parser;
use image::{DynamicImage, ImageFormat, ImageReader, ImageResult};
use std::{
    fs,
    io::{self, Write},
    ops::{Div, Mul},
    path::{Path, PathBuf},
};

use color_histogram::*;
use dhash::*;

mod color_histogram;
mod dhash;

// some extensions (eg. `riff`) will error out, even though the format can be guessed by the contents
pub fn open_img_with_guessed_fmt(
    p: &Path,
) -> ImageResult<(DynamicImage, ImageFormat)> {
    let reader = ImageReader::new(io::BufReader::new(fs::File::open(p)?))
        .with_guessed_format()?;
    let fmt = reader.format().unwrap();
    let img = reader.decode()?;
    Ok((img, fmt))
}

const MAX_WIDTH: u32 = 256;
const MAX_HEIGHT: u32 = 256;

pub fn list_pics(folder: &Path) -> Vec<PathBuf> {
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
    fn dist(
        &self,
        other: &Self,
        histogram_weight: f32,
        dhash_weight: f32,
    ) -> f32 {
        histogram_weight * self.histogram.dist_normalized(&other.histogram)
            + dhash_weight * self.dhash.dist_normalized(&other.dhash)
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

fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    s
}

fn sort_images(args: &CliArgs, output: &Path) {
    let paths = list_pics(&args.input);
    let max_digits = paths.len().ilog10() as usize + 1;

    let mut hashes: Vec<_> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            print!("Processing image {i:0>max_digits$} of {}\r", paths.len());
            let img = open_img_with_guessed_fmt(p).unwrap().0;
            let img = img.resize(
                MAX_WIDTH,
                MAX_HEIGHT,
                image::imageops::FilterType::Triangle,
            );
            (p, ImgHash::from(&img))
        })
        .collect();
    println!();

    println!("Sorting images...");
    let mut sorted = vec![hashes.remove(0)];
    while !hashes.is_empty() {
        let last = &sorted.last().unwrap().1;
        let (idx, _) = hashes
            .iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| {
                last.dist(a, args.histogram_weight, args.dhash_weight)
                    .partial_cmp(&last.dist(
                        b,
                        args.histogram_weight,
                        args.dhash_weight,
                    ))
                    .unwrap()
            })
            .unwrap();
        sorted.push(hashes.remove(idx));
    }

    if !fs::exists(output).unwrap() {
        fs::create_dir_all(output).unwrap();
    }
    let in_place = args.input == output;

    // create output
    for (new_i, (old, _)) in sorted.iter().enumerate() {
        let mut old_i = usize::MAX;

        let new = if in_place {
            // prefix with generation-ID in case of multiple retries
            let mut old_gen = u32::MAX;
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
            output
                .join(format!("{new_gen}_{new_i:0>max_digits$}"))
                .with_extension(old.extension().unwrap_or_default())
        } else {
            output
                .join(format!("{new_i:0>max_digits$}"))
                .with_extension(old.extension().unwrap_or_default())
        };

        if fs::exists(&new).unwrap() {
            if in_place {
                println!("skipping {old:?} => {new:?} (already exists)");
                continue;
            } else {
                let s =
                    prompt(&format!("{new:?} already exists. Overwrite? y/N "));
                if s.trim().to_lowercase() != "y" {
                    continue;
                }
            }
        }

        if !args.quiet && old_i != new_i {
            println!(
                "{:?} => {:?}",
                old.file_name().unwrap(),
                new.file_name().unwrap()
            );
        }
        if !args.test {
            fs::rename(old, new).unwrap();
        }
    }
}

#[derive(Parser)]
struct CliArgs {
    /// Input directory
    input: PathBuf,
    /// Output directory
    output: Option<PathBuf>,
    #[arg(long, default_value = "0.5")]
    /// Weight of the color histogram - how much color-similarity matters
    histogram_weight: f32,
    /// Weight of the dhash - how much shape-similarity matters
    #[arg(long, default_value = "0.5")]
    dhash_weight: f32,
    /// Quiet mode - don't print renames
    #[arg(long)]
    quiet: bool,
    /// Test mode - don't apply changes
    #[arg(short, long)]
    test: bool,
}

fn main() {
    let args = CliArgs::parse();
    let output = args.output.as_ref().unwrap_or(&args.input);
    sort_images(&args, output);
}

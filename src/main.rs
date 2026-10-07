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

    /// compare `self`'s with `other`'s similarity to reference
    fn cmp_to_reference(
        &self,
        other: &Self,
        reference: &Self,
        histogram_weight: f32,
        dhash_weight: f32,
    ) -> std::cmp::Ordering {
        reference
            .dist(self, histogram_weight, dhash_weight)
            .partial_cmp(&reference.dist(other, histogram_weight, dhash_weight))
            .unwrap()
    }
}

impl From<DynamicImage> for ImgHash {
    fn from(img: image::DynamicImage) -> Self {
        const MAX_WIDTH: u32 = 256;
        const MAX_HEIGHT: u32 = 256;
        // shrinking twice from full size is needlessly expensive, and we're never gonna need full size
        // so histogram doesn't resize at all, and dhash resizes from this
        let img = img.resize(
            MAX_WIDTH,
            MAX_HEIGHT,
            image::imageops::FilterType::Triangle,
        );
        Self {
            histogram: Histogram::from(&img),
            dhash: DHash::from(&img),
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

fn load_hash(p: &Path) -> ImgHash {
    let img = open_img_with_guessed_fmt(p).unwrap().0;
    ImgHash::from(img)
}

#[derive(Clone, Copy)]
struct GenName {
    generation: u32,
    idx: usize,
}

impl GenName {
    fn next_gen(self, idx: usize) -> Self {
        Self {
            generation: self.generation.wrapping_add(1),
            idx,
        }
    }

    fn as_filename(self, max_digits: usize) -> String {
        format!("{}_{:0>max_digits$}", self.generation, self.idx)
    }
}

impl TryFrom<&Path> for GenName {
    type Error = ();
    fn try_from(p: &Path) -> Result<Self, Self::Error> {
        p.file_stem()
            .unwrap()
            .to_str()
            .and_then(|s| s.split_once('_'))
            .and_then(|(gen_s, idx_s)| {
                gen_s
                    .parse::<u32>()
                    .and_then(|generation| {
                        idx_s
                            .parse::<usize>()
                            .map(|idx| Self { generation, idx })
                    })
                    .ok()
            })
            .ok_or(())
    }
}

fn sort_pictures(
    inputs: &[PathBuf],
    output_dir: &Path,
    reference: &Path,
    args: &CliArgs,
) {
    let max_digits = inputs.len().ilog10() as usize + 1;
    let mut hashes: Vec<_> = inputs
        .iter()
        .enumerate()
        .map(|(i, p)| {
            print!("Processing image {i:0>max_digits$} of {}\r", inputs.len());
            let hash = load_hash(p);
            (p.as_path(), hash)
        })
        .collect();
    println!();

    let norm_reference =
        reference.canonicalize().unwrap_or(reference.to_owned());

    let (elem0, remove0) = if let Some(pos) = inputs.iter().position(|p| {
        p.canonicalize().as_deref().unwrap_or(p) == norm_reference
    }) {
        (hashes.remove(pos), false)
    } else {
        let hash = load_hash(&norm_reference);
        ((&*norm_reference, hash), true)
    };

    println!("Sorting images...");
    let mut sorted = vec![elem0];
    while !hashes.is_empty() {
        let last = &sorted.last().unwrap().1;
        let (idx, _) = hashes
            .iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| {
                a.cmp_to_reference(
                    b,
                    last,
                    args.histogram_weight,
                    args.dhash_weight,
                )
            })
            .unwrap();
        sorted.push(hashes.remove(idx));
    }

    if remove0 {
        sorted.remove(0);
    }

    if !fs::exists(output_dir).unwrap() {
        fs::create_dir_all(output_dir).unwrap();
    }
    let in_place = args.input == output_dir;

    // create output
    for (new_idx, (old, _)) in sorted.into_iter().enumerate() {
        let ext = old.extension().unwrap_or_default();
        let old_gen = GenName::try_from(old);
        let new_name = if let Ok(old_gen) = old_gen {
            let new_gen = old_gen.next_gen(new_idx);
            new_gen.as_filename(max_digits)
        } else {
            format!("{new_idx:0>max_digits$}")
        };
        let new = output_dir.join(new_name).with_extension(ext);

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

        if !args.quiet
            && (old_gen.is_err() || old_gen.is_ok_and(|g| g.idx != new_idx))
        {
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

    /// Reference image to start the comparison chain from.
    /// Defaults to a random one, making the result slightly different each time.
    #[arg(long)]
    reference: Option<PathBuf>,

    /// Weight of the color histogram - how much color-similarity matters
    #[arg(long, default_value = "0.5")]
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
    let output_dir = args.output.as_ref().unwrap_or(&args.input);
    let inputs = list_pics(&args.input);
    if let Some(first) = inputs.first() {
        let reference = args.reference.as_ref().unwrap_or(first);
        sort_pictures(&inputs, output_dir, reference, &args);
    } else {
        println!("No images found in {:?}", args.input);
    }
}

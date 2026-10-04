use std::{cmp::Ordering, io::Write, *};

use color_histogram::*;
use picsort::*;

mod color_histogram;

const DIR_ORIGINAL: &str = "original";
const DIR_OUT: &str = "out";
const MAX_WIDTH: u32 = 256;
const MAX_HEIGHT: u32 = 256;
const OUT_WIDTH: u32 = 32;
const OUT_HEIGHT: u32 = 32;

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
        path::PathBuf::from(DIR_ORIGINAL)
            .join("notes")
            .join("initial_rename.log"),
    )
    .unwrap();
    log.write_all(b"[\n").unwrap();
    let pics = list_pics_natural_sort(DIR_ORIGINAL);
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

fn main() {
    let pics = list_pics(DIR_ORIGINAL);
    let imgs_with_fmts: Vec<_> = pics
        .iter()
        .map(|p| {
            let (img, fmt) = open_img_with_guessed_fmt(p).unwrap();
            let img = img.resize(
                MAX_WIDTH,
                MAX_HEIGHT,
                image::imageops::FilterType::Triangle,
            );
            (img, fmt)
        })
        .collect();

    let mut histograms: Vec<_> = imgs_with_fmts
        .iter()
        .enumerate()
        .map(|(i, (img, _))| (i, Histogram::from(img)))
        .collect();

    // sort histograms
    let mut sorted = vec![histograms.remove(0)];
    while !histograms.is_empty() {
        let last = &sorted.last().unwrap().1;
        let (idx, _) = histograms
            .iter()
            .enumerate()
            .min_by(|(_, (_, a)), (_, (_, b))| {
                last.dist(a).partial_cmp(&last.dist(b)).unwrap()
            })
            .unwrap();
        sorted.push(histograms.remove(idx));
    }
    assert_eq!(pics.len(), sorted.len());

    // create reordered result
    fs::create_dir_all(DIR_OUT).unwrap();
    let max_digits = num_digits(pics.len());
    for (new_i, (old_i, _)) in sorted.iter().enumerate() {
        let old_path = &pics[*old_i];
        let (img, fmt) = &imgs_with_fmts[*old_i];
        let ext = old_path.extension().unwrap().to_string_lossy();
        let new_path = path::PathBuf::from(DIR_OUT)
            .join(format!("{new_i:0>max_digits$}.{ext}"));
        let downsampled = img.resize(
            OUT_WIDTH,
            OUT_HEIGHT,
            image::imageops::FilterType::Triangle,
        );
        downsampled.save_with_format(&new_path, *fmt).unwrap();
        //fs::copy(old_path, &new_path).unwrap();
    }
}

use std::cmp::max;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{self, Read};

struct Image<T> {
    width: u32,
    height: u32,
    data: Vec<T>,
}

impl<T> Image<T> {
    fn new(width: u32, height: u32, data: Vec<T>) -> Self {
        let size = (width * height) as usize;
        assert!(size == data.len());
        Image {
            width,
            height,
            data: data,
        }
    }

    // Print to screen, use conversion function to convert T to u8 grayscale
    fn print_with_conversion<F: Fn(&T) -> u8>(&self, convert: F) {
        for y in 0..self.height {
            for x in 0..self.width {
                let gray = convert(&self.data[(y * self.width + x) as usize]);
                print!("\x1b[48;2;{};{};{}m  ", gray, gray, gray);
            }
            print!("\x1b[0m\n");
        }
    }
}

type MnistImage = Image<u8>;

fn maybe_report_magic_mismatch(filename: &str, actual: u32, expected: u32) -> std::io::Result<()> {
    if expected != actual {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{}: Unexpected magic number; Got {:#08x}; expected {:#08x}",
                filename, actual, expected
            ),
        ));
    }
    Ok(())
}

fn maybe_report_unexpected_filesize(
    filename: &str,
    file: &File,
    expected_size: usize,
) -> std::io::Result<()> {
    let actual_filesize = file.metadata()?.len() as usize;
    if actual_filesize != expected_size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{}: Unexpected file size; expected {}, got {}",
                filename, expected_size, actual_filesize
            ),
        ));
    }
    Ok(())
}

fn read_be_u32(file: &mut File) -> std::io::Result<u32> {
    let mut buffer = vec![0; 4];
    file.read_exact(&mut buffer)?;
    Ok(u32::from_be_bytes(buffer[0..4].try_into().unwrap()))
}

fn read_labels(filename: &str) -> std::io::Result<Vec<u8>> {
    const LABEL_MAGIC_NUMBER: u32 = 0x801;
    let mut file = File::open(filename)?;
    let magic = read_be_u32(&mut file)?;
    maybe_report_magic_mismatch(filename, magic, LABEL_MAGIC_NUMBER)?;
    let count = read_be_u32(&mut file)? as usize;
    let expected_filesize = 8 + count;
    maybe_report_unexpected_filesize(filename, &file, expected_filesize)?;

    let mut result = vec![0; count];
    file.read_exact(&mut result)?;
    return Ok(result);
}

fn read_images(filename: &str) -> std::io::Result<Vec<MnistImage>> {
    const IMAGE_MAGIC_NUMBER: u32 = 0x803;
    let mut file = File::open(filename)?;
    let magic = read_be_u32(&mut file)?;
    maybe_report_magic_mismatch(filename, magic, IMAGE_MAGIC_NUMBER)?;
    let count = read_be_u32(&mut file)? as usize;
    let rows = read_be_u32(&mut file)?;
    let columns = read_be_u32(&mut file)?;
    let expected_filesize = 16 + count * (rows * columns) as usize;
    maybe_report_unexpected_filesize(filename, &file, expected_filesize)?;

    let image_size = (columns * rows) as usize;
    let mut result: Vec<MnistImage> = Vec::new();
    for _ in 0..count {
        let mut data = vec![0; image_size];
        file.read_exact(&mut data)?;
        result.push(MnistImage::new(columns, rows, data));
    }
    return Ok(result);
}

fn usage() -> std::io::Result<()> {
    println!("Usage: handwriting-detect-rs <labels-file> <image-file>\n");
    return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "expected arguments",
    ));
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return usage();
    }
    let labels = read_labels(&args[1])?;
    let images = read_images(&args[2])?;

    println!("Getting {} labels, {} images", labels.len(), images.len());
    if labels.len() != images.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "labels vs. image mismatch",
        ));
    }

    // Sum up all the images for the corresponding labels to get an 'average'
    // image.
    type SumImage = Image<u32>;
    let mut label2sum: BTreeMap<u8, SumImage> = BTreeMap::new();
    for i in 0..labels.len() {
        let label = &labels[i];
        let image = &images[i];
        if !label2sum.contains_key(label) {
            label2sum.insert(
                *label,
                SumImage::new(image.width, image.height, vec![0; image.data.len()]),
            );
        }
        if let Some(s) = label2sum.get_mut(label) {
            for pixel in 0..image.data.len() {
                s.data[pixel] += image.data[pixel] as u32;
            }
        }
    }

    for (label, image) in &label2sum {
        println!("Label: {} ------------------------------\n", label);
        let mut max_value: u32 = 0;
        for val in image.data.iter() {
	    max_value = max(max_value, *val);
	}
        image.print_with_conversion(|value| (255 * value / max_value) as u8);
    }


    Ok(())
}

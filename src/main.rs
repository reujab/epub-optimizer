extern crate tempfile;
extern crate walkdir;
extern crate zip;

use std::env;
use std::fs::File;
use std::fs;
use std::io::Write;
use std::io;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let mut bytes_saved = 0;
    let args = env::args().skip(1);
    for path in args {
        println!("{}:", path);
        bytes_saved += process(path);
    }
    println!("{}KiB saved in total.", bytes_saved / 1024);
}

fn process(path: String) -> u64 {
    println!("Reading ZIP...");
    let file = File::open(&path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();

    println!("Extracting ZIP...");
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..zip.len() {
        let mut input = zip.by_index(i).unwrap();
        if input.name().ends_with("/") {
            continue;
        }
        let input_path = input.sanitized_name();

        let output_path = tmp.path().join(input_path);
        let _ = fs::create_dir_all(output_path.parent().unwrap());
        let mut output = File::create(output_path).unwrap();

        io::copy(&mut input, &mut output).unwrap();
    }

    println!("Minifying files...");
    let mut bytes_saved = 0;
    for entry in WalkDir::new(&tmp) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();

        let ext = path.extension();
        if ext == None {
            continue;
        }
        let ext = ext.unwrap();

        let original_len = entry.metadata().unwrap().len();
        match ext.to_str().unwrap() {
            "html" | "htm" | "css" | "svg" | "xml" => {
                Command::new("minify").
                    arg(path).
                    arg("-o").
                    arg(path).
                    output().
                    unwrap();
            }
            "jpeg" | "jpg" => {
                Command::new("jpegoptim").
                    arg("-s").
                    arg(path).
                    output().
                    unwrap();
            }
            "png" => {
                Command::new("crunch").
                    arg(path).
                    output().
                    unwrap();
                // FIXME when crunch adds an option to overwrite file
                // https://github.com/chrissimpkins/Crunch/issues/20
                fs::rename(path.parent().unwrap().join(path.file_stem().unwrap().to_str().unwrap().to_owned() + "-crunch.png"), path).unwrap();
            }
            _ => {}
        }
        bytes_saved += original_len - entry.metadata().unwrap().len();
        print!("\r{}KiB saved.", bytes_saved / 1024);
        io::stdout().flush().unwrap();
    }
    println!();

    println!();
    bytes_saved
}

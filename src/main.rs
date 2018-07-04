extern crate tempfile;
extern crate zip;

use std::env;
use std::fs::File;
use std::fs;
use std::io;

fn main() {
    let args = env::args().skip(1);
    for path in args {
        println!("{}:", path);
        process(path);
    }
}

fn process(path: String) {
    println!("Reading ZIP...");
    let file = File::open(path).unwrap();
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
}

extern crate zip;

use std::env;
use std::fs::File;

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

    println!("Reading container...");
    let container = zip.by_name("META-INF/container.xml").unwrap();
}

extern crate tempfile;
extern crate walkdir;
extern crate xmltree;
extern crate zip;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::io;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let mut bytes_saved: i64 = 0;
    let args = env::args().skip(1);
    for path in args {
        println!("{}:", path);
        let original_len = fs::metadata(&path).unwrap().len() as i64;
        process(&path);
        let optimized_len = fs::metadata(&path).unwrap().len() as i64;
        bytes_saved += original_len - optimized_len;

        println!();
    }
    println!("{}KiB saved in total.", bytes_saved / 1024);
}

fn process(path: &String) {
    let tmp = unzip(&path);
    mod_metadata(&tmp);
    minify(&tmp);
    gen_epub(&path, &tmp);
}

fn unzip(path: &String) -> tempfile::TempDir {
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

    tmp
}

struct Metadata {
    cover_id: String,

    language: String,
    title: String,
    creator: String,
    subjects: Vec<String>,
    date: String,
    description: String,
}

fn mod_metadata(tmp: &tempfile::TempDir) {
    println!("Rewriting metadata...");
    let file = File::open(format!("{}/META-INF/container.xml", tmp.path().to_str().unwrap())).unwrap();
    let doc = xmltree::Element::parse(file).unwrap();
    let opf = &doc.
        get_child("rootfiles").unwrap().
        get_child("rootfile").unwrap().
        attributes["full-path"];

    let mut file = File::open(tmp.path().join(&opf)).unwrap();
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    let mut doc = xmltree::Element::parse(str::replace(buffer.as_str(), "\u{feff}", "").as_bytes()).unwrap();

    {
        let metadata_ele = doc.get_mut_child("metadata").unwrap();
        let mut metadata = Metadata{
            cover_id: String::new(),

            language: String::new(),
            title: String::new(),
            creator: String::new(),
            subjects: vec![],
            date: String::new(),
            description: String::new(),
        };
        for child in &metadata_ele.children {
            if child.name == "meta" {
                let key = child.attributes.get("name").unwrap_or(&String::new()).clone();
                let val = child.attributes.get("content").unwrap_or(&String::new()).clone();
                println!("{}: {}={}", child.name, key, val);

                if key == "cover" {
                    metadata.cover_id = val;
                }
            } else {
                let key = &child.name;
                let val = &child.text.clone().unwrap_or(String::new());
                let val = val.clone();
                println!("{}: {}", key, val);
                match key.as_str() {
                    "language" => metadata.language = val,
                    "title" => metadata.title = val,
                    "creator" => metadata.creator = val,
                    "subject" => metadata.subjects.push(val),
                    "date" => metadata.date = val[0..4].to_owned(),
                    "description" => metadata.description = val,
                    _ => {}
                }
            }
        }

        metadata_ele.children = vec![];
        if metadata.cover_id.len() != 0 {
            let mut ele = xmltree::Element::new("meta");
            ele.attributes = HashMap::with_capacity(2);
            ele.attributes.insert("name".to_owned(), "cover".to_owned());
            ele.attributes.insert("content".to_owned(), metadata.cover_id);
            metadata_ele.children.push(ele);
        }
        fn prompt(dom: &mut xmltree::Element, name: &str, default: String) {
            let mut input = String::new();
            print!("{} [{}]: ", name, default);
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            let mut input = input.trim().to_owned();
            if input.len() == 0 {
                input = default;
            }

            let mut ele = xmltree::Element::new(name);
            ele.prefix = Some("dc".to_owned());
            ele.text = Some(input);
            dom.children.push(ele);
        }
        prompt(metadata_ele, "language", metadata.language);
        prompt(metadata_ele, "title", metadata.title);
        prompt(metadata_ele, "creator", metadata.creator);
        prompt(metadata_ele, "subject", if metadata.subjects.len() == 0 { String::new() } else { metadata.subjects[0].clone() });
        prompt(metadata_ele, "date", metadata.date);
        prompt(metadata_ele, "description", metadata.description);
    }

    let file = File::create(tmp.path().join(&opf)).unwrap();
    doc.write(file).unwrap();
}

fn minify(tmp: &tempfile::TempDir) {
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
            "opf" | "xml" | "html" | "htm" => {
                Command::new("minify").
                    arg("--mime=text/xml").
                    arg(path).
                    output().
                    unwrap();
            }
            "css" | "svg" => {
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
}

fn gen_epub(path: &String, tmp: &tempfile::TempDir) {
    println!("Zipping...");
    let wd = env::current_dir().unwrap();
    let path_abs = fs::canonicalize(&path).unwrap();

    let _ = fs::remove_file(&path);
    env::set_current_dir(&tmp).unwrap();
    let mut cmd = Command::new("zip");
    cmd.arg("-9r");
    cmd.arg(&path_abs);
    for path in fs::read_dir(".").unwrap() {
        cmd.arg(path.unwrap().path());
    }
    cmd.output().unwrap();
    env::set_current_dir(wd).unwrap();
}

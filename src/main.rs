extern crate toml;
extern crate rustc_serialize;
extern crate regex;
extern crate ansi_term;

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use toml::Value;
use rustc_serialize::json::Json;
use regex::Regex;
use ansi_term::Colour::*;

fn exec_command(command: &str, args: &[&str]) -> (String, String) {
    let output = Command::new(command)
                         .args(args)
                         .output()
                         .expect(&format!("Executing command {}", command));

    if !output.status.success() {
        println!("{}", std::str::from_utf8(&output.stdout).unwrap_or("Decoding stdout failed."));
        println!("{}", std::str::from_utf8(&output.stderr).unwrap_or("Decoding stderr failed."));
        panic!("Error occurred in {} {}", command, args.join(" "));
    }

    (String::from_utf8(output.stdout)
            .expect(&format!("Decoding stdout failed in {} {}", command, args.join(" "))),
     String::from_utf8(output.stderr)
            .expect(&format!("Decoding stderr failed in {} {}", command, args.join(" "))))
}

fn exec_cargo(args: &[&str]) -> (String, String) {
    exec_command("cargo", args)
}

fn location() -> String {
    let (out, _)= exec_cargo(&["locate-project"]);
    let data = Json::from_str(&out).unwrap();
    let s = data.as_object().unwrap().get("root").unwrap().as_string().unwrap();
    String::from(s)
}

macro_rules! load_config {
    ($root:expr, $config:expr, $name:ident) => {
        if let Some(tmp) = $root.get(stringify!($name)) {
            $config.$name = tmp.as_str().expect(&format!("Invalid config for {}", stringify!($name))).to_owned();
        }
    }
}

macro_rules! load_config_option {
    ($root:expr, $config:expr, $name:ident) => {
        if let Some(tmp) = $root.get(stringify!($name)) {
            $config.$name = Some(tmp.as_str().expect(&format!("Invalid config for {}", stringify!($name))).to_owned());
        }
    }
}

fn load_config(path: &str) -> Config {
    let mut toml_file = File::open(path).expect(&format!("Unable to open {}", path));
    let mut t = String::new();

    toml_file.read_to_string(&mut t).expect(&format!("Unable to read {}", path));

    let toml_config = t.parse::<Value>().expect(&format!("Unable to parse {}", path));
    let mut config = Config { node: NODE.to_owned(), target: TARGET.to_owned(), prelude: None };

    if let Some(testjs) = toml_config["package"]["metadata"].get("testjs") {
        load_config!(testjs, config, node);
        load_config!(testjs, config, target);
        load_config_option!(testjs, config, prelude);
    }

    config
}

const TARGET: &'static str = "asmjs-unknown-emscripten";
const NODE: &'static str = "node";
#[derive(Default, Debug)]
struct Config {
    node: String,
    target: String,
    prelude: Option<String>,
}

fn find_test_jss(proj_root: &Path, target: &str) -> Vec<PathBuf> {
    let re = Regex::new(r"^[^-]+-[0-9a-f]+\.js$").unwrap();
    let test_dir = proj_root.join("target").join(target).join("debug");
    let files = test_dir.read_dir().expect(&format!("Directory {} not found",
                                                    test_dir.to_str()
                                                            .expect("Cannot decode the directory name")));

    files.filter_map(|f| f.ok())
         .filter_map(|f| f.file_name().into_string().ok())
         .filter(|f| re.is_match(f))
         .map(|f| test_dir.join(f))
         .collect::<Vec<PathBuf>>()
}

fn exec_nodejs(nodepath: &str, script: &str) {
    let mut node = Command::new(nodepath)
                           .stdin(Stdio::piped())
                           .stdout(Stdio::inherit())
                           .stderr(Stdio::inherit())
                           .spawn().expect("Cannot execute node");

    node.stdin
        .take()
        .expect("Cannot execute the test script from stdin")
        .write_all(script.as_bytes())
        .expect("Cannot execute the test script from stdin");

    node.wait()
        .expect(&format!("{} did not finish properly", nodepath));
}

fn main() {
    let toml_location = location();
    let proj_root = Path::new(&toml_location).parent().unwrap();
    let config = load_config(&toml_location);
    let cargo_args = ["test", "--target", &config.target, "--no-run"];

    println!("{}", Blue.paint("Compiling JS Test..."));
    println!("{}", Green.paint(format!("cargo {}", cargo_args.join(" "))));
    Command::new("cargo")
            .args(&cargo_args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn().expect("Cannot execute cargo")
            .wait().expect(&format!("Command cargo {} did not finish properly", cargo_args.join(" ")));

    let mut files = find_test_jss(&proj_root, &config.target);
    let path = if files.len() == 1 {
        files[0].as_path()
    } else {
        println!("{}", Red.paint("Multiple js test file has found."));
        files.sort_by_key(|ref f| f.metadata().expect("The filesystem does not support metadata")
                                              .modified()
                                              .expect("The filesystem does not support timestamp"));
        println!("{}", Red.paint(format!("Use the latest one, {}",
                                         files[files.len() - 1].to_str().expect("Cannot decode the filename"))));
        files[files.len() - 1].as_path()
    };

    println!("{}", Blue.paint("Running node..."));

    let mut scripts = String::new();

    if let Some(path) = config.prelude {
        let mut pre_file = File::open(proj_root.join(&path)).expect(&format!("Unable to open {}", path));
        pre_file.read_to_string(&mut scripts).expect(&format!("Unable to read {}", path));
        scripts.push('\n');
    }

    let mut main_file = File::open(path).expect(&format!("Unable to open {}",
                                                         path.to_str().expect("Cannot decode the filename")));
    main_file.read_to_string(&mut scripts).expect(&format!("Unable to read {}",
                                                           path.to_str().expect("Cannot decode the filename")));

    exec_nodejs(&config.node, &scripts);
}

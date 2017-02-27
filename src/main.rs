#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate rustc_serialize;
extern crate regex;

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use rustc_serialize::json::Json;
use regex::Regex;

fn exec_command(command: &str, args: &[&str]) -> String {
    let output = Command::new(command)
                         .args(args)
                         .output()
                         .expect(&format!("Executing command {}", command));

    if !output.status.success() {
        println!("{}", std::str::from_utf8(&output.stdout).unwrap_or("Decoding stdout failed."));
        println!("{}", std::str::from_utf8(&output.stderr).unwrap_or("Decoding stderr failed."));
        panic!("Error occurred in {} {}", command, args.join(" "));
    }

    String::from_utf8(output.stdout)
           .expect(&format!("Decoding stdout failed in {} {}", command, args.join(" ")))
}

fn exec_cargo(args: &[&str]) -> String {
    exec_command("cargo", args)
}

fn location() -> String {
    let data = Json::from_str(&exec_cargo(&["locate-project"])).unwrap();
    let s = data.as_object().unwrap().get("root").unwrap().as_string().unwrap();
    String::from(s)
}

fn load_config(path: &str) -> Config {
    let mut toml_file = File::open(path).expect(&format!("Unable to open {}", path));
    let mut t = String::new();

    toml_file.read_to_string(&mut t).expect(&format!("Unable to read {}", path));

    let root: RootConfig = toml::from_str(&t).expect(&format!("Unable to parse {}", path));

    if let Some(c) = root.testjs {
        c
    } else {
        Default::default()
    }
}

#[derive(Debug, Deserialize)]
struct RootConfig {
    testjs: Option<Config>,
}

const TARGET: &'static str = "asmjs-unknown-emscripten";
#[derive(Default, Debug, Deserialize)]
struct Config {
    target: Option<String>,
    prelude: Option<String>,
}

fn find_test_jss(proj_root: &Path, target: &str) -> Vec<PathBuf> {
    let re = Regex::new(r"^[^-]+-[0-9a-f]+\.js$").unwrap();
    let test_dir = proj_root.join("target").join(target).join("debug");
    let files = test_dir.read_dir().expect(&format!("Directory {} not found",
                                                    test_dir.to_str()
                                                            .expect("Cannot decode the directory name")));

    files.filter(|f| f.is_ok())
         .map(|f| f.unwrap().file_name().into_string())
         .filter(|f| f.is_ok())
         .map(|f| f.unwrap())
         .filter(|f| re.is_match(f))
         .map(|f| test_dir.join(f))
         .collect::<Vec<PathBuf>>()
}

fn exec_nodejs(script: &str) {
    // TODO: option for binary file and error handling
    let mut node = Command::new("nodejs")
                           .stdin(Stdio::piped())
                           .stdout(Stdio::inherit())
                           .stderr(Stdio::inherit())
                           .spawn().expect("Cannot execute node");
    node.stdin.take().unwrap().write_all(script.as_bytes());
    node.wait();
}

fn main() {
    let toml_location = location();
    let proj_root = Path::new(&toml_location).parent().unwrap();
    let config = load_config(&toml_location);
    let target = if let Some(ref t) = config.target { t.as_str() } else { TARGET };
    let cargo_args = ["test", "--target", target, "--no-run"];

    println!("cargo {}", cargo_args.join(" "));
    println!("{}", exec_cargo(&cargo_args));
    println!("Compiling the test has done.");

    let mut files = find_test_jss(&proj_root, target);
    let path = if files.len() == 1 {
        files[0].as_path()
    } else {
        println!("Multiple js test file has found.");
        files.sort_by_key(|ref f| f.metadata().expect("The filesystem does not support metadata")
                                              .modified()
                                              .expect("The filesystem does not support timestamp"));
        println!("Use the latest one, {}", files[files.len() - 1].to_str().expect("Cannot decode the filename"));
        files[files.len() - 1].as_path()
    };

    let mut scripts = String::new();

    if let Some(path) = config.prelude {
        let mut pre_file = File::open(&path).expect(&format!("Unable to open {}", path));
        pre_file.read_to_string(&mut scripts).expect(&format!("Unable to read {}", path));
        scripts.push('\n');
    }

    let mut main_file = File::open(path).expect(&format!("Unable to open {}",
                                                         path.to_str().expect("Cannot decode the filename")));
    main_file.read_to_string(&mut scripts).expect(&format!("Unable to read {}",
                                                           path.to_str().expect("Cannot decode the filename")));

    exec_nodejs(&scripts);
}

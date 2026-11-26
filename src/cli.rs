use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use clap::Parser;

pub static CONFIG_PATHS: &'static [&'static str] = &[
    env!("AITA_STREAMER_GLOBAL_CONFIG_PATH"),
    env!("AITA_STREAMER_USER_CONFIG_PATH"),
];

// Run RTPS server
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

impl Cli {
    fn read_config_file(&self, path: PathBuf) -> String {
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents);
        contents
    }
    pub fn get_config_str(&self) -> String {
        self.read_config_file(match &self.config {
            Some(path) => {
                if !path.exists() {
                    panic!("Config path {} does not exist", path.display());
                }
                if !path.is_file() {
                    panic!("Config path {} does not aim to file", path.display());
                }
                print!("Using config {}", path.display());
                path.clone()
            }
            None => 'outer: loop {
                for p in CONFIG_PATHS {
                    let mut path = PathBuf::new();
                    path.push(p);
                    if path.exists() && path.is_file() {
                        print!("Using config {}", path.display());
                        break 'outer path;
                    }
                }
                panic!("No config file found");
            },
        })
    }
}

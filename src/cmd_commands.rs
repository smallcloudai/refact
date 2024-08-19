use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use crate::caps::SIMPLE_CAPS;
use crate::global_context::CommandLine;

fn generate_byok_file(cache_dir: &PathBuf) {
    let file_name = cache_dir.join("custom_caps.yaml");
    let mut file = fs::File::create(file_name).expect("Failed to create file");
    file.write_all(SIMPLE_CAPS.as_ref()).expect("Failed to write to file");
}

pub fn exec_commands_if_exists(cache_dir: &PathBuf) {
    let cmdline = CommandLine::from_args();
    if cmdline.save_byok_file {
        generate_byok_file(&cache_dir);
        exit(0);
    }
}
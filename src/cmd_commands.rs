use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use crate::caps::SIMPLE_CAPS;
use crate::global_context::CommandLine;

fn generate_byok_file(file_name: &PathBuf) {
    let mut file = fs::File::create(file_name).expect("Failed to create file");
    file.write_all(SIMPLE_CAPS.as_ref()).expect("Failed to write to file");
}

pub fn exec_commands_if_exists(cache_dir: &PathBuf) {
    let cmdline = CommandLine::from_args();
    if cmdline.save_byok_file {
        let file_name = cache_dir.join("bring-your-own-key.yaml");
        generate_byok_file(&file_name);
        println!("{}", file_name.display());
        exit(0);
    }
}

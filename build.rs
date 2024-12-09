use std::env;
use std::fs::{self, read_dir, File};
use std::io::Write;


fn main() -> shadow_rs::SdResult<()> {
    let assets_dir = "assets/integrations";
    let out_dir = env::var("OUT_DIR").unwrap();
    let output_file_path = format!("{}/available_icons.rs", out_dir);
    let mut output_file = File::create(&output_file_path).expect("Failed to create output file");

    writeln!(
        output_file,
        "use std::collections::HashMap;\n\npub fn get_available_icons() -> HashMap<&'static str, &'static [u8]> {{\n    let mut icons = HashMap::new();"
    )
        .expect("Failed to write to output file");

    for entry in read_dir(assets_dir).expect("Failed to read assets directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.extension().and_then(|ext| ext.to_str()) == Some("png") {
            let image_data = fs::read(&path).expect("Failed to read image file");
            let file_stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("Failed to get file stem");

            let constant_name = format!("{}_ICON_BYTES", file_stem.to_uppercase());

            writeln!(
                output_file,
                "    pub const {}: &[u8] = &{:?};",
                constant_name, image_data
            )
                .expect("Failed to write constant definition");

            writeln!(
                output_file,
                "    icons.insert(\"{}.png\", {});",
                file_stem, constant_name
            )
                .expect("Failed to write HashMap entry");
        }
    }

    writeln!(output_file, "    icons\n}}").expect("Failed to write closing brace");

    shadow_rs::new()
}

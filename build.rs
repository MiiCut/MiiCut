use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let examples_dir = Path::new("examples");
    println!("cargo:rerun-if-changed={}", examples_dir.display());

    let mut example_files = Vec::new();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_mii_json(&path) {
                println!("cargo:rerun-if-changed={}", path.display());
                example_files.push(path);
            }
        }
    }
    example_files.sort();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = out_dir.join("examples_gen.rs");
    let mut output = String::new();
    output.push_str("pub(crate) static EXAMPLES: &[(&str, &str)] = &[\n");
    for path in &example_files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .trim_end_matches(".mii.json");
        let rel_path = path.strip_prefix(".").unwrap_or(path);
        let rel = rel_path.to_string_lossy();
        output.push_str("    (");
        output.push_str(&format!("{name:?}, "));
        output.push_str(&format!(
            "include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}\"))),\n",
            rel
        ));
    }
    output.push_str("];\n");

    fs::write(out_file, output).unwrap();
}

fn is_mii_json(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with(".mii.json")
}

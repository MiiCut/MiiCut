use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    write_examples(&out_dir, "examples", "EXAMPLES", "examples_gen.rs");
    write_examples(&out_dir, "tutorial", "TUTORIALS", "tutorial_gen.rs");
}

fn write_examples(out_dir: &Path, folder: &str, const_name: &str, out_name: &str) {
    let dir = Path::new(folder);
    println!("cargo:rerun-if-changed={}", dir.display());

    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_json(&path) {
                println!("cargo:rerun-if-changed={}", path.display());
                files.push(path);
            }
        }
    }
    files.sort();

    let out_file = out_dir.join(out_name);
    let mut output = String::new();
    output.push_str(&format!(
        "pub(crate) static {const_name}: &[(&str, &str)] = &[\n"
    ));
    for path in &files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .trim_end_matches(".json");
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

fn is_json(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.ends_with(".json")
}

use std::{env, fs, path::PathBuf};

fn main() {
    let data = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("data");
    println!("cargo::rerun-if-changed={}", data.display());

    let mut files: Vec<PathBuf> = fs::read_dir(&data)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();

    let mut generated = String::from("pub static DATASETS: &[&str] = &[\n");
    for file in &files {
        generated.push_str(&format!("    include_str!({:?}),\n", file.display()));
    }
    generated.push_str("];\n");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("datasets.rs");
    fs::write(&out, generated).expect("write datasets.rs");
}

use std::{env, fs, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../crates/yue-component-icons/assets/tabler");
    let mut code = String::from("{\n");
    for style in ["outline", "filled"] {
        let dir = root.join(style);
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "svg"))
            .collect();
        files.sort();
        for file in files {
            let name: String = file
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .split('-')
                .map(|part| {
                    let mut chars = part.chars();
                    format!(
                        "{}{}",
                        chars.next().unwrap().to_ascii_uppercase(),
                        chars.collect::<String>()
                    )
                })
                .collect();
            code.push_str(&format!("std::hint::black_box({style}::{name}.svg());\n"));
        }
    }
    code.push('}');
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("all-icons.rs"),
        code,
    )
    .unwrap();
}

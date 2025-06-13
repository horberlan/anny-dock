use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/*");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let assets_path = Path::new("assets");
    let dest_assets_path = dest_path.join("assets");

    if dest_assets_path.exists() {
        fs::remove_dir_all(&dest_assets_path).unwrap();
    }

    fs::create_dir_all(&dest_assets_path).unwrap();

    copy_dir_all(assets_path, &dest_assets_path).unwrap();
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_file = dst.join(entry.file_name());
        if ty.is_dir() {
            fs::create_dir_all(&dest_file)?;
            copy_dir_all(&entry.path(), &dest_file)?;
        } else {
            fs::copy(entry.path(), &dest_file)?;
        }
    }
    Ok(())
}

use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn inputs(root: &Path) -> io::Result<Vec<PathBuf>> {
    fn collect(root: &Path, path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                collect(root, &path, files)?;
            } else {
                files.push(path.strip_prefix(root).unwrap().to_path_buf());
            }
        }
        Ok(())
    }
    let mut files = ["Cargo.toml", "Cargo.lock", "build.rs", "build_support.rs"]
        .map(PathBuf::from)
        .to_vec();
    for directory in ["src", "schemas/mcp", "fonts"] {
        collect(root, &root.join(directory), &mut files)?;
    }
    files.sort();
    Ok(files)
}

pub fn fingerprint(root: &Path, files: &[PathBuf]) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut files = files.to_vec();
    files.sort();
    for path in files {
        let name = path.to_str().unwrap().replace('\\', "/");
        let bytes = fs::read(root.join(&path))?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

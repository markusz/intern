use std::fs;
use std::path::{Path, PathBuf};

use miette::{Context, IntoDiagnostic};

/// Expands the inputs into a flat list of presentation files: a file is kept
/// as-is, a directory is replaced by the `.pptx` files directly inside it
/// (sorted, for a deterministic order).
pub fn collect_pptx(inputs: &[PathBuf]) -> miette::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for input in inputs {
        if input.is_dir() {
            out.extend(pptx_in_dir(input)?);
        } else {
            out.push(input.clone());
        }
    }
    Ok(out)
}

fn pptx_in_dir(dir: &Path) -> miette::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(dir)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot read directory '{}'", dir.display()))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_pptx(path))
        .collect();
    found.sort();
    Ok(found)
}

fn is_pptx(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(ext) if ext.eq_ignore_ascii_case("pptx")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_pptx_matches_case_insensitively() {
        assert!(is_pptx(Path::new("deck.pptx")));
        assert!(is_pptx(Path::new("deck.PPTX")));
        assert!(!is_pptx(Path::new("deck.txt")));
        assert!(!is_pptx(Path::new("deck")));
    }

    #[test]
    fn collect_pptx_passes_files_through_and_expands_dirs() {
        let base = std::env::temp_dir();
        let dir = base.join("intern_input_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("b.pptx"), b"x").unwrap();
        fs::write(dir.join("a.pptx"), b"x").unwrap();
        fs::write(dir.join("notes.txt"), b"x").unwrap();
        let loose = base.join("intern_input_loose.pptx");
        fs::write(&loose, b"x").unwrap();

        let got = collect_pptx(&[dir.clone(), loose.clone()]).unwrap();

        fs::remove_dir_all(&dir).ok();
        fs::remove_file(&loose).ok();

        // Directory expanded to its sorted .pptx files; the loose file passed through.
        assert_eq!(got, vec![dir.join("a.pptx"), dir.join("b.pptx"), loose]);
    }
}

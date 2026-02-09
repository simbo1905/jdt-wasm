use pretty_assertions::assert_eq;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn read_json(path: &Path) -> Value {
    let s = fs::read_to_string(path).unwrap();
    let s = jdt_codegen::strip_bom(&s);
    serde_json::from_str(s).unwrap()
}

fn find_source_file(transform_file: &Path, prefix: &str) -> PathBuf {
    let mut dir = transform_file.parent().unwrap().to_path_buf();
    loop {
        let candidate = dir.join(format!("{prefix}.Source.json"));
        if candidate.exists() {
            return candidate;
        }
        if !dir.pop() {
            panic!("could not find {prefix}.Source.json for {transform_file:?}");
        }
    }
}

#[test]
fn microsoft_fixture_suite() {
    let inputs_dir = std::env::var("JDT_MS_INPUTS_DIR")
        .expect("set JDT_MS_INPUTS_DIR (use `xmake run test_all`)");
    let inputs_dir = PathBuf::from(inputs_dir);

    let mut cases = 0usize;
    let mut skipped = 0usize;
    for entry in WalkDir::new(&inputs_dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.contains(".Transform") || !name.ends_with(".json") {
            continue;
        }

        // Upstream fixture inconsistency (as of the pinned commit): expected output references
        // source values not present in any checked-in Source.json. Keep this skipped until
        // the upstream suite is corrected.
        if path
            .to_string_lossy()
            .ends_with("Rename/Array.ScriptPath.Transform .json")
        {
            skipped += 1;
            continue;
        }

        // Match Expected file by replacing ".Transform" with ".Expected".
        // Some upstream fixtures have a stray space before ".json" in the Transform filename.
        let mut expected_name = name.replace(".Transform", ".Expected");
        expected_name = expected_name.replace(" .json", ".json");
        let expected_path = path.with_file_name(expected_name);
        assert!(expected_path.exists(), "missing expected file for {path:?}");

        let prefix = name.split('.').next().unwrap();
        let source_path = find_source_file(path, prefix);

        let source = read_json(&source_path);
        let transform = read_json(path);
        let expected = read_json(&expected_path);

        let actual = jdt_codegen::apply(&source, &transform).unwrap();
        assert_eq!(expected, actual, "case failed: {}", path.display());
        cases += 1;
    }

    assert!(
        cases > 0,
        "no test cases discovered under {}",
        inputs_dir.display()
    );
    eprintln!("ms suite: ran {cases} cases, skipped {skipped}");
}

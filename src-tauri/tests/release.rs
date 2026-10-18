//! Facts that must agree before a release is tagged.

use std::path::PathBuf;

fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn json_version(relative: &str) -> String {
    let json: serde_json::Value = serde_json::from_str(&read(relative)).unwrap();
    json["version"].as_str().unwrap().to_string()
}

#[test]
fn every_manifest_names_the_same_version() {
    let cargo = env!("CARGO_PKG_VERSION");
    assert_eq!(json_version("tauri.conf.json"), cargo, "tauri.conf.json");
    assert_eq!(json_version("../package.json"), cargo, "package.json");
}

#[test]
fn the_release_workflow_builds_what_the_cask_installs() {
    let workflow = read("../.github/workflows/release.yml");
    let cask = read("../packaging/homebrew/growth-notch.rb");
    assert!(workflow.contains("aarch64-apple-darwin") && workflow.contains("x86_64-apple-darwin"));
    // The cask asks for GrowthNotch_<version>_<arch>.dmg, where the arch is aarch64 or x64.
    assert!(cask.contains(r#"arch arm: "aarch64", intel: "x64""#));
    assert!(cask.contains("GrowthNotch_#{version}_#{arch}.dmg"));
    assert!(cask.contains(&format!("version \"{}\"", env!("CARGO_PKG_VERSION"))));
}

#[test]
fn third_party_actions_are_pinned_to_commits() {
    for workflow in ["ci.yml", "release.yml"] {
        let text = read(&format!("../.github/workflows/{workflow}"));
        for line in text
            .lines()
            .filter(|l| l.trim_start().starts_with("- uses:"))
        {
            let reference = line.split('@').nth(1).unwrap_or_default();
            let sha = reference.split_whitespace().next().unwrap_or_default();
            assert!(
                sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()),
                "{workflow}: `{}` is not pinned to a full commit SHA",
                line.trim()
            );
        }
    }
}

#[test]
fn the_app_starts_without_a_dock_icon() {
    // Setting the activation policy in code runs about a second after launch, so the plist must
    // already say this app is an agent.
    let plist = read("Info.plist");
    let key = plist
        .find("<key>LSUIElement</key>")
        .expect("LSUIElement is missing");
    assert!(plist[key..]
        .trim_start_matches("<key>LSUIElement</key>")
        .trim_start()
        .starts_with("<true/>"));
}

/// The files that make up the project, found by walking the folders that hold them.
fn project_files() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for dir in [
        "src",
        "src-tauri/src",
        "src-tauri/tests",
        "src-tauri/capabilities",
        "src-tauri/icons",
        ".github",
        "docs",
        "assets",
        "packaging",
    ] {
        walk(&root.join(dir), &mut files);
    }
    // docs/launch holds personal notes. It is ignored on purpose and never pushed.
    files.retain(|path| !path.to_string_lossy().contains("docs/launch"));
    for file in [
        "package.json",
        "package-lock.json",
        "src-tauri/Cargo.toml",
        "src-tauri/Cargo.lock",
        "src-tauri/Info.plist",
        "src-tauri/build.rs",
        "src-tauri/tauri.conf.json",
        "LICENSE",
        "README.md",
    ] {
        files.push(root.join(file));
    }
    files
}

#[test]
fn no_project_file_is_hidden_by_gitignore() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // A rule like `vault.*` once hid `vault.rs`, which would have left the vault tests out of the repo.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let Ok(mut git) = Command::new("git")
        .args(["check-ignore", "--no-index", "--stdin"])
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return; // no git here, for example when building from a source archive
    };

    let paths: Vec<String> = project_files()
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .filter_map(|path| {
            let relative = path
                .strip_prefix(root.canonicalize().ok()?)
                .ok()?
                .to_path_buf();
            Some(relative.to_string_lossy().into_owned())
        })
        .collect();
    git.stdin
        .take()
        .unwrap()
        .write_all(paths.join("\n").as_bytes())
        .unwrap();
    let output = git.wait_with_output().unwrap();
    if output.status.code() == Some(128) {
        return; // not a git checkout
    }
    let hidden = String::from_utf8_lossy(&output.stdout);
    assert!(hidden.trim().is_empty(), "ignored by .gitignore:\n{hidden}");
}

#[test]
fn no_file_holds_something_shaped_like_a_real_secret() {
    // Each marker is assembled from pieces so that this file does not contain the thing it looks for.
    let private_key = ["-----", "BEGIN "].concat();
    let followed_by = |text: &str, prefix: &str, min_len: usize| {
        text.match_indices(prefix).any(|(at, _)| {
            let rest = &text[at + prefix.len()..];
            rest.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .count()
                >= min_len
        })
    };

    for path in project_files() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // binary files such as images
        };
        let name = path.display();
        assert!(
            !followed_by(&text, "AIza", 35),
            "{name}: looks like a Google API key"
        );
        assert!(
            !followed_by(&text, "github_pat_", 20),
            "{name}: looks like a GitHub token"
        );
        for prefix in ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"] {
            assert!(
                !followed_by(&text, prefix, 30),
                "{name}: looks like a GitHub token"
            );
        }
        assert!(
            !followed_by(&text, "AKIA", 16),
            "{name}: looks like an AWS key"
        );
        assert!(
            !(text.contains(&private_key) && text.contains("PRIVATE KEY-----")),
            "{name}: contains a private key"
        );
    }
}

#[test]
fn the_docs_state_the_minimum_versions_the_manifests_require() {
    let cargo = read("Cargo.toml");
    let rust = cargo
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = "))
        .expect("Cargo.toml has no rust-version")
        .trim_matches('"')
        .to_string();
    let package: serde_json::Value = serde_json::from_str(&read("../package.json")).unwrap();
    let node = package["engines"]["node"]
        .as_str()
        .expect("package.json has no engines.node")
        .trim_start_matches(">=")
        .trim_end_matches(".0")
        .to_string();

    for doc in ["../README.md", "../CONTRIBUTING.md"] {
        let text = read(doc);
        assert!(
            text.contains(&format!("Node {node} or newer")),
            "{doc} does not say Node {node}"
        );
        assert!(
            text.contains(&format!("Rust {rust} or newer")),
            "{doc} does not say Rust {rust}"
        );
    }
}

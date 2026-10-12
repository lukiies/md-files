//! Golden test against the BAU contract templates and their configuration
//! copies (`egzemplarze-konfiguracyjne/`): every part of every stored copy
//! must equal what `itamd::compile` produces from the template with the
//! copy's facts.
//!
//! The templates are confidential and live in another repository, so the
//! test runs only when `ITAMD_BAU_DIR` points at the folder that holds the
//! `*.szablon.md` files (with `egzemplarze-konfiguracyjne/` inside):
//!
//! ```text
//! ITAMD_BAU_DIR="…/umowy-komplet-do-podpisu-2026-07" cargo test -p md-core --test bau_golden -- --nocapture
//! ```
//!
//! The BAU generator wraps the standard's passes with its own steps, which
//! this file reproduces only to compare like with like: the client body of a
//! template (between the change history banner and the dependency footer,
//! without the acceptance lines), and after the passes the removal of
//! operator hints `*( … )*` and of link targets.

use std::path::{Path, PathBuf};

use md_core::itamd::{cleanup, parse_facts, project, CompileOptions};
use md_core::json::{self, Json};

fn bau_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("ITAMD_BAU_DIR")?);
    dir.join("egzemplarze-konfiguracyjne").is_dir().then_some(dir)
}

/// BAU `template_body`: the client body of a template.
fn template_body(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut start = lines.iter().position(|l| l.trim().eq_ignore_ascii_case("### change history")).unwrap_or(0);
    if let Some(j) = (start..lines.len()).find(|&j| lines[j].starts_with("> ")) {
        start = j;
    }
    let end = (start..lines.len()).find(|&j| lines[j].starts_with("## Related documents")).unwrap_or(lines.len());
    let mut body: Vec<&str> = lines[start..end].to_vec();
    let skip = body
        .iter()
        .take_while(|l| l.starts_with('>') || l.trim().is_empty() || l.trim() == "---")
        .count();
    body.drain(..skip);
    while body.last().is_some_and(|l| l.trim().is_empty() || l.trim() == "---") {
        body.pop();
    }
    // BAU DROP_LINE: acceptance lines go to the document's acceptance block.
    body.into_iter()
        .filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("**Akceptacja:")
                || t.starts_with("**Potwierdzenie doręczenia")
                || t.starts_with("**Rozpoczęcie świadczenia (JDG):"))
        })
        .map(str::to_string)
        .collect()
}

/// BAU `HINT`: operator hints `*( … )*` (never bold, never `*(a)*`).
fn remove_hints(t: &str) -> String {
    let b: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '*' && b.get(i + 1) == Some(&'(') && (i == 0 || b[i - 1] != '*') {
            let lettered = {
                let mut k = i + 2;
                if b.get(k).is_some_and(|c| c.is_ascii_lowercase() || *c == 'W') {
                    k += 1;
                    if b.get(k).is_some_and(char::is_ascii_digit) {
                        k += 1;
                    }
                    b.get(k) == Some(&')') && b.get(k + 1) == Some(&'*')
                } else {
                    false
                }
            };
            if !lettered {
                if let Some(j) = (i + 2..b.len()).find(|&j| b[j] == ')') {
                    if b.get(j + 1) == Some(&'*') && b.get(j + 2) != Some(&'*') {
                        i = j + 2;
                        continue;
                    }
                }
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// BAU `LINK`: `[text](target)` becomes `text`.
fn remove_link_targets(t: &str) -> String {
    let b: Vec<char> = t.chars().collect();
    let mut out = String::with_capacity(t.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '[' {
            if let Some(c) = (i + 1..b.len()).find(|&k| b[k] == ']') {
                if c > i + 1 && b.get(c + 1) == Some(&'(') {
                    if let Some(e) = (c + 2..b.len()).find(|&k| b[k] == ')') {
                        if e > c + 2 {
                            out.extend(&b[i + 1..c]);
                            i = e + 1;
                            continue;
                        }
                    }
                }
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// The text of each part of a composed copy, by part number.
fn copy_parts(doc: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = doc.split('\n').collect();
    let heads: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].starts_with("## Część ") || lines[i] == "## Akceptacja")
        .collect();
    let mut parts = Vec::new();
    for w in heads.windows(2) {
        let (h, next) = (w[0], w[1]);
        let nr = lines[h]["## Część ".len()..].split(' ').next().unwrap_or("").to_string();
        // Composition: heading, blank, text, then "", "---", "" before the next heading.
        parts.push((nr, lines[h + 2..next - 3].join("\n")));
    }
    parts
}

#[test]
fn bau_copies_equal_compiled_templates() {
    let Some(dir) = bau_dir() else {
        eprintln!("bau_golden: ITAMD_BAU_DIR not set — skipped (see the file header)");
        return;
    };
    let samples = dir.join("egzemplarze-konfiguracyjne");
    let mut manifests: Vec<PathBuf> = std::fs::read_dir(&samples)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".manifest.json"))
        .collect();
    manifests.sort();
    assert!(!manifests.is_empty(), "no manifests in {}", samples.display());
    let mut compared = 0;
    let mut failures = Vec::new();
    for manifest_path in &manifests {
        let manifest_text = std::fs::read_to_string(manifest_path).unwrap();
        let manifest = json::parse(&manifest_text).unwrap();
        let facts = parse_facts(&manifest_text).unwrap();
        let copy_path = PathBuf::from(manifest_path.to_string_lossy().replace(".manifest.json", ".md"));
        let doc = std::fs::read_to_string(&copy_path).unwrap().replace("\r\n", "\n");
        let parts = copy_parts(&doc);
        let Some(Json::Array(listed)) = manifest.get("parts") else { panic!("no parts in manifest") };
        for part in listed {
            let nr = part.get("part").and_then(Json::as_str).unwrap();
            let template = part.get("template").and_then(Json::as_str).unwrap();
            let source = std::fs::read_to_string(dir.join(template)).unwrap().replace("\r\n", "\n");
            let body = template_body(&source).join("\n");
            let projected = match project(&body, &facts, &CompileOptions::default()) {
                Ok((t, _)) => t,
                Err(e) => {
                    failures.push(format!("{}: {template}: {e:?}", name(&copy_path)));
                    continue;
                }
            };
            let got = cleanup(&remove_link_targets(&remove_hints(&projected)));
            let expected = parts.iter().find(|(n, _)| n == nr).map(|(_, t)| t.as_str()).unwrap_or("<missing part>");
            compared += 1;
            if got.trim_end() != expected {
                let line = got.lines().zip(expected.lines()).position(|(a, b)| a != b).map(|k| k + 1);
                failures.push(format!("{}: part {nr} ({template}) differs, first at line {line:?}", name(&copy_path)));
            }
        }
    }
    eprintln!("bau_golden: {compared} parts compared, {} differences", failures.len());
    assert!(failures.is_empty(), "{failures:#?}");
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

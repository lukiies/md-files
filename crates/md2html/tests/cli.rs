//! Integration tests for the md2html CLI: run the real binary end to end and
//! check the HTML it produces.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn md2html() -> Command {
    Command::new(env!("CARGO_BIN_EXE_md2html"))
}

/// A per-test scratch folder (removed on drop, best-effort).
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("md2html_test_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn write(&self, name: &str, content: &[u8]) -> PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn help_prints_usage_and_succeeds() {
    let out = md2html().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(stdout_str(&out).contains("Usage:"));
}

#[test]
fn converts_file_to_full_page() {
    let dir = Scratch::new("full_page");
    let md = dir.write("doc.md", b"# Title\n\nSome **bold** text.\n");
    let out = md2html().arg(&md).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let html = stdout_str(&out);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<strong>bold</strong>"));
    // Default title is the input filename.
    assert!(html.contains("<title>doc.md</title>"));
}

#[test]
fn body_flag_outputs_fragment_only() {
    let dir = Scratch::new("body_only");
    let md = dir.write("doc.md", b"# Title\n");
    let out = md2html().arg("--body").arg(&md).output().unwrap();
    assert!(out.status.success());
    let html = stdout_str(&out);
    assert!(html.contains("<h1>Title</h1>"));
    assert!(!html.contains("<!DOCTYPE html>"));
}

#[test]
fn reads_stdin_when_no_file_given() {
    let mut child = md2html()
        .arg("--body")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"*emphasis*\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert!(stdout_str(&out).contains("<em>emphasis</em>"));
}

#[test]
fn out_flag_writes_file() {
    let dir = Scratch::new("out_flag");
    let md = dir.write("doc.md", b"# Out\n");
    let target = dir.path().join("doc.html");
    let out = md2html().arg(&md).arg("-o").arg(&target).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let html = std::fs::read_to_string(&target).unwrap();
    assert!(html.contains("<h1>Out</h1>"));
}

#[test]
fn dark_and_title_flags_change_page() {
    let dir = Scratch::new("dark_title");
    let md = dir.write("doc.md", b"text\n");
    let out = md2html()
        .arg("--dark")
        .arg("--title")
        .arg("My Page")
        .arg(&md)
        .output()
        .unwrap();
    assert!(out.status.success());
    let html = stdout_str(&out);
    assert!(html.contains("<title>My Page</title>"));
    assert!(html.contains("#1e1e1e")); // dark background
}

#[test]
fn code_block_uses_cdn_by_default_and_inline_when_embedded() {
    let dir = Scratch::new("assets_mode");
    let md = dir.write("doc.md", b"```rust\nfn main() {}\n```\n");

    let cdn = md2html().arg(&md).output().unwrap();
    assert!(cdn.status.success());
    let cdn_html = stdout_str(&cdn);
    assert!(cdn_html.contains("<script src=\"https://cdnjs.cloudflare.com"));

    let embedded = md2html().arg("--embed").arg(&md).output().unwrap();
    assert!(embedded.status.success());
    let embedded_html = stdout_str(&embedded);
    assert!(!embedded_html.contains("<script src="));
    assert!(embedded_html.contains("hljs"));
}

#[test]
fn mermaid_fence_becomes_diagram_container() {
    let dir = Scratch::new("mermaid");
    let md = dir.write("doc.md", b"```mermaid\nflowchart TD\nA-->B\n```\n");
    let out = md2html().arg(&md).output().unwrap();
    assert!(out.status.success());
    let html = stdout_str(&out);
    assert!(html.contains("<pre class=\"mermaid\">"));
    assert!(html.contains("mermaid"));
}

#[test]
fn embed_inlines_local_images_as_data_uris() {
    let dir = Scratch::new("embed_img");
    // A minimal valid PNG header is enough — only the bytes are inlined.
    dir.write("pic.png", b"\x89PNG\r\n\x1a\n0000");
    let md = dir.write("doc.md", b"![p](pic.png)\n");
    let out = md2html().arg("--embed").arg(&md).output().unwrap();
    assert!(out.status.success());
    let html = stdout_str(&out);
    assert!(
        html.contains("src=\"data:image/png;base64,"),
        "image was not inlined: {html}"
    );
}

#[test]
fn without_embed_image_paths_stay_as_authored() {
    let dir = Scratch::new("plain_img");
    dir.write("pic.png", b"\x89PNG\r\n\x1a\n0000");
    let md = dir.write("doc.md", b"![p](pic.png)\n");
    let out = md2html().arg(&md).output().unwrap();
    assert!(out.status.success());
    assert!(stdout_str(&out).contains("src=\"pic.png\""));
}

#[test]
fn external_image_survives_embed_untouched() {
    let dir = Scratch::new("embed_ext_img");
    let md = dir.write("doc.md", b"![x](https://example.com/a.png)\n");
    let out = md2html().arg("--embed").arg(&md).output().unwrap();
    assert!(out.status.success());
    assert!(stdout_str(&out).contains("src=\"https://example.com/a.png\""));
}

#[test]
fn save_writes_html_next_to_input() {
    let dir = Scratch::new("save_flag");
    let md = dir.write("doc.md", b"# Saved\n");
    let out = md2html().arg("--save").arg(&md).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let html = std::fs::read_to_string(dir.path().join("doc.html")).unwrap();
    assert!(html.contains("<h1>Saved</h1>"));
}

#[test]
fn save_without_file_input_fails() {
    let out = md2html().arg("--save").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("--save needs a FILE input"));
}

#[test]
fn explicit_out_wins_over_save() {
    let dir = Scratch::new("save_vs_out");
    let md = dir.write("doc.md", b"# X\n");
    let target = dir.path().join("custom.html");
    let out = md2html().arg("--save").arg(&md).arg("-o").arg(&target).output().unwrap();
    assert!(out.status.success());
    assert!(target.is_file());
    assert!(!dir.path().join("doc.html").exists());
}

#[test]
fn notes_kept_by_default_stripped_on_request() {
    let dir = Scratch::new("strip_notes");
    let md = dir.write(
        "doc.md",
        "# T\n\n> \u{1f7e2} **[DRAFTED]** working note\n\nProse.\n<!-- hidden -->\n".as_bytes(),
    );

    // Default: a viewer-style conversion shows the note.
    let plain = md2html().arg("--body").arg(&md).output().unwrap();
    assert!(plain.status.success());
    assert!(stdout_str(&plain).contains("DRAFTED"));

    // --strip-notes: deliverable preview, note and comment gone.
    let stripped = md2html().arg("--body").arg("--strip-notes").arg(&md).output().unwrap();
    assert!(stripped.status.success());
    let html = stdout_str(&stripped);
    assert!(!html.contains("DRAFTED"), "got: {html}");
    assert!(!html.contains("hidden"));
    assert!(html.contains("Prose."));
}

#[test]
fn unknown_option_fails_with_message() {
    let out = md2html().arg("--nope").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("unknown option"));
}

#[test]
fn missing_input_file_fails() {
    let out = md2html().arg("definitely_missing_file.md").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("cannot read"));
}

#[test]
fn missing_option_values_fail() {
    for flag in ["--out", "--title"] {
        let out = md2html().arg(flag).output().unwrap();
        assert!(!out.status.success(), "{flag} with no value should fail");
        assert!(stderr_str(&out).contains("missing value"));
    }
}

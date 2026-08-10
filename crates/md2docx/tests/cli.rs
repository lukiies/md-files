//! Integration tests for the md2docx CLI. Error paths run everywhere; the
//! real conversion tests run only where pandoc is installed (they are skipped
//! with a note otherwise, since pandoc is an external runtime dependency).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn md2docx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_md2docx"))
}

fn pandoc_available() -> bool {
    Command::new("pandoc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::env::var("LOCALAPPDATA")
            .map(|l| Path::new(&l).join("Pandoc").join("pandoc.exe").is_file())
            .unwrap_or(false)
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("md2docx_test_{name}"));
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

fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn help_prints_usage_and_succeeds() {
    let out = md2docx().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage:"));
}

#[test]
fn no_input_fails() {
    let out = md2docx().output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("no input file"));
}

#[test]
fn missing_input_fails() {
    let out = md2docx().arg("definitely_missing.md").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("input not found"));
}

#[test]
fn unknown_option_fails() {
    let out = md2docx().arg("--nope").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("unknown option"));
}

#[test]
fn missing_reference_doc_fails() {
    let dir = Scratch::new("bad_ref");
    let md = dir.write("doc.md", b"# Hi\n");
    let out = md2docx()
        .arg(&md)
        .arg("--reference-doc")
        .arg(dir.path().join("nope.docx"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("reference doc not found"));
}

#[test]
fn bad_pandoc_path_fails_cleanly() {
    let dir = Scratch::new("bad_pandoc");
    let md = dir.write("doc.md", b"# Hi\n");
    let out = md2docx()
        .arg(&md)
        .arg("--pandoc")
        .arg(dir.path().join("no_such_pandoc.exe"))
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("cannot run pandoc"));
}

#[test]
fn converts_markdown_to_docx() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("convert");
    let md = dir.write("report.md", b"# Report\n\nHello **world**.\n");
    let out = md2docx().arg(&md).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let docx = dir.path().join("report.docx");
    assert!(docx.is_file(), "expected {} to exist", docx.display());
    // A .docx is a ZIP container: check the magic bytes, not just existence.
    let bytes = std::fs::read(&docx).unwrap();
    assert!(bytes.starts_with(b"PK"), "output is not a ZIP/docx container");
}

#[test]
fn out_flag_controls_output_path() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("out_path");
    let md = dir.write("notes.md", b"# Notes\n");
    let target = dir.path().join("Final Report.docx");
    let out = md2docx().arg(&md).arg("-o").arg(&target).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    assert!(target.is_file());
}

/// Extract the visible text of a docx by asking pandoc to convert it back.
fn docx_text(docx: &Path) -> String {
    let out = Command::new("pandoc")
        .arg(docx)
        .args(["--to", "plain"])
        .output()
        .unwrap();
    assert!(out.status.success(), "pandoc docx->plain failed");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const NOTED_SOURCE: &[u8] = "# Report\n\n\
> \u{1f7e2} **[DRAFTED]** intro complete\n\
> still the same note\n\n\
Real paragraph one.\n\n\
<!-- hidden working comment -->\n\
[EXPANSION NOTE: add graphs]\n\n\
> A genuine quotation to keep.\n\n\
Real paragraph two. TBC (numbers)\n"
    .as_bytes();

#[test]
fn strips_working_notes_by_default() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("strip_notes");
    let md = dir.write("report.md", NOTED_SOURCE);
    let out = md2docx().arg(&md).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let text = docx_text(&dir.path().join("report.docx"));
    assert!(!text.contains("DRAFTED"), "note leaked into docx: {text}");
    assert!(!text.contains("still the same note"));
    assert!(!text.contains("hidden working comment"));
    assert!(!text.contains("EXPANSION NOTE"));
    assert!(text.contains("Real paragraph one."));
    assert!(text.contains("genuine quotation"), "real blockquote lost: {text}");
    assert!(text.contains("TBC"), "TBC placeholder must survive interim builds");
}

#[test]
fn keep_notes_converts_verbatim() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("keep_notes");
    let md = dir.write("report.md", NOTED_SOURCE);
    let out = md2docx().arg(&md).arg("--keep-notes").output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let text = docx_text(&dir.path().join("report.docx"));
    assert!(text.contains("DRAFTED"), "--keep-notes should keep notes: {text}");
}

#[test]
fn final_refuses_while_tbc_remains() {
    let dir = Scratch::new("final_tbc");
    let md = dir.write("report.md", NOTED_SOURCE);
    let out = md2docx().arg(&md).arg("--final").output().unwrap();
    assert!(!out.status.success());
    assert!(stderr_str(&out).contains("REFUSING"), "got: {}", stderr_str(&out));
    assert!(!dir.path().join("report.docx").exists());
}

#[test]
fn final_converts_when_source_is_clean() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("final_clean");
    let md = dir.write(
        "report.md",
        "# Report\n\n> \u{1f7e2} **[DRAFTED]** done\n\nAll finished.\n".as_bytes(),
    );
    let out = md2docx().arg(&md).arg("--final").output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    assert!(dir.path().join("report.docx").is_file());
}

#[test]
fn tbc_inside_stripped_note_does_not_block_final() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("final_note_tbc");
    // The TBC lives only inside a working note; after stripping the
    // deliverable is clean, so --final must go through.
    let md = dir.write(
        "report.md",
        "# Report\n\n> \u{1f4dd} **[WORKING NOTE]** numbers TBC here\n\nDone text.\n".as_bytes(),
    );
    let out = md2docx().arg(&md).arg("--final").output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
}

#[test]
fn toc_marker_note_enables_table_of_contents() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("toc_marker");
    let md = dir.write(
        "report.md",
        "> \u{1f9ed} **[TABLE OF CONTENTS]** auto\n\n# One\n\ntext\n\n# Two\n\ntext\n".as_bytes(),
    );
    let out = md2docx().arg(&md).output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    let text = docx_text(&dir.path().join("report.docx"));
    assert!(!text.contains("TABLE OF CONTENTS"), "marker leaked: {text}");
    // pandoc represents a docx TOC as a native Word field under a
    // "Table of Contents" heading (Word fills the entries on open), so the
    // heading is what proves --toc was enabled by the marker.
    assert!(text.contains("Table of Contents"), "no TOC generated: {text}");
}

#[test]
fn toc_flag_is_accepted() {
    if !pandoc_available() {
        eprintln!("SKIPPED: pandoc not installed");
        return;
    }
    let dir = Scratch::new("toc");
    let md = dir.write("doc.md", b"# One\n\ntext\n\n# Two\n\ntext\n");
    let out = md2docx().arg(&md).arg("--toc").output().unwrap();
    assert!(out.status.success(), "stderr: {}", stderr_str(&out));
    assert!(dir.path().join("doc.docx").is_file());
}

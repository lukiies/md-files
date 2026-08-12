//! md2docx: convert a markdown file to a Word document.
//!
//! A cross-platform front-end for pandoc — the same conversion pipeline used
//! for the MSc report builds — with sane defaults: the input's folder is the
//! resource path (so relative images resolve), and pandoc is located even
//! when it is installed but not on PATH (the default Windows per-user
//! installer location, plus common Unix prefixes).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
md2docx - convert Markdown to a Word document (via pandoc)

The .md file is treated as the editable SOURCE and the .docx as the compiled
deliverable: visible working notes (any blockquote opening with an emoji
marker, e.g. \"> \u{1f7e2} **[DRAFTED]** ...\"), HTML comments and legacy
[DRAFTED]/[EXPANSION NOTE] lines are stripped before conversion, so they
never reach the document. A \"> \u{1f9ed} **[TABLE OF CONTENTS]**\" marker
note turns into a pandoc table of contents at conversion time.

Usage:
  md2docx [OPTIONS] FILE

Arguments:
  FILE                  Markdown file to convert

Options:
  -o, --out FILE        Output .docx path (default: input name with .docx)
  --reference-doc FILE  Style the output like this reference .docx (pandoc
                        --reference-doc: fonts, heading styles, margins)
  --toc                 Insert a table of contents
  --keep-notes          Convert the source verbatim - do not strip notes,
                        HTML comments or placeholders markers
  --final               Deliverable must be clean: refuse to convert while
                        any TBC placeholder remains in the source
  --pandoc PATH         Use this pandoc executable
  -h, --help            Show this help

Examples:
  md2docx report.md
  md2docx report.md -o \"Final Report.docx\" --reference-doc styles.docx
  md2docx \"report (editable source).md\" -o \"report (final).docx\" --final
";

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut reference_doc: Option<PathBuf> = None;
    let mut toc = false;
    let mut keep_notes = false;
    let mut final_stage = false;
    let mut pandoc_override: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-o" | "--out" => match args.next() {
                Some(v) => output = Some(PathBuf::from(v)),
                None => return fail("missing value for --out"),
            },
            "--reference-doc" => match args.next() {
                Some(v) => reference_doc = Some(PathBuf::from(v)),
                None => return fail("missing value for --reference-doc"),
            },
            "--toc" => toc = true,
            "--keep-notes" => keep_notes = true,
            "--final" => final_stage = true,
            "--pandoc" => match args.next() {
                Some(v) => pandoc_override = Some(PathBuf::from(v)),
                None => return fail("missing value for --pandoc"),
            },
            _ if arg.starts_with('-') => return fail(&format!("unknown option: {arg}")),
            _ => {
                if input.is_some() {
                    return fail("more than one input file given");
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    let Some(input) = input else {
        return fail("no input file given");
    };
    if !input.is_file() {
        return fail(&format!("input not found: {}", input.display()));
    }

    let output = output.unwrap_or_else(|| input.with_extension("docx"));

    // All user input is validated before pandoc is even looked for, so a
    // machine without pandoc still reports what is actually wrong with the
    // invocation instead of masking it behind the install instructions.
    if let Some(reference) = &reference_doc {
        if !reference.is_file() {
            return fail(&format!("reference doc not found: {}", reference.display()));
        }
    }

    // Relative images in the document resolve against the input's folder.
    let resource_dir = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    // Source -> deliverable: strip working notes unless asked not to. The
    // cleaned text goes through a temp file; --resource-path keeps relative
    // images resolving against the real input's folder.
    let mut temp_input: Option<PathBuf> = None;
    let pandoc_input = if keep_notes {
        if final_stage {
            let text = match std::fs::read_to_string(&input) {
                Ok(t) => t,
                Err(e) => return fail(&format!("cannot read {}: {e}", input.display())),
            };
            if let Some(code) = refuse_final(&text) {
                return code;
            }
        }
        input.clone()
    } else {
        let text = match std::fs::read_to_string(&input) {
            Ok(t) => t,
            Err(e) => return fail(&format!("cannot read {}: {e}", input.display())),
        };
        let stripped = md_core::strip_source_notes(&text);
        if final_stage {
            if let Some(code) = refuse_final(&stripped.text) {
                return code;
            }
        }
        if stripped.wants_toc {
            toc = true;
        }
        if stripped.wants_figures_list || stripped.wants_tables_list {
            eprintln!(
                "md2docx: note - [TABLE OF FIGURES]/[LIST OF TABLES] markers were \
                 stripped; auto-updatable Word fields need a docx post-processing \
                 step, which pandoc cannot produce."
            );
        }
        let temp = std::env::temp_dir().join(format!("md2docx-{}.md", std::process::id()));
        if let Err(e) = std::fs::write(&temp, &stripped.text) {
            return fail(&format!("cannot write temp file {}: {e}", temp.display()));
        }
        temp_input = Some(temp.clone());
        temp
    };

    let Some(pandoc) = pandoc_override.or_else(find_pandoc) else {
        if let Some(temp) = &temp_input {
            let _ = std::fs::remove_file(temp);
        }
        eprintln!("md2docx: pandoc not found.");
        eprintln!();
        eprintln!("Install it and retry:");
        eprintln!("  Windows: winget install --id JohnMacFarlane.Pandoc");
        eprintln!("  macOS:   brew install pandoc");
        eprintln!("  Linux:   sudo apt install pandoc   (or your distro's package)");
        eprintln!();
        eprintln!("Or point at an existing install with --pandoc <path>.");
        return ExitCode::FAILURE;
    };

    let mut cmd = Command::new(&pandoc);
    cmd.arg(&pandoc_input)
        .arg("-o")
        .arg(&output)
        .args(["--from", "markdown", "--to", "docx"])
        .arg("--resource-path")
        .arg(&resource_dir);
    if toc {
        cmd.arg("--toc");
    }
    if let Some(reference) = &reference_doc {
        cmd.arg("--reference-doc").arg(reference);
    }

    let result = cmd.status();
    if let Some(temp) = &temp_input {
        let _ = std::fs::remove_file(temp);
    }

    match result {
        Ok(status) if status.success() => {
            println!("wrote {}", output.display());
            ExitCode::SUCCESS
        }
        Ok(status) => {
            eprintln!("md2docx: pandoc failed (exit code {:?})", status.code());
            ExitCode::FAILURE
        }
        Err(e) => fail(&format!("cannot run pandoc ({}): {e}", pandoc.display())),
    }
}

/// The --final guard: a deliverable marked final must contain no TBC
/// placeholders. Returns the failure exit code if any remain.
fn refuse_final(text: &str) -> Option<ExitCode> {
    let tbc = md_core::count_tbc(text);
    if tbc > 0 {
        eprintln!(
            "md2docx: REFUSING to convert as final: {tbc} TBC placeholder(s) \
             remain in the source."
        );
        return Some(ExitCode::FAILURE);
    }
    None
}

/// Locate pandoc: PATH first, then the locations the installers use.
fn find_pandoc() -> Option<PathBuf> {
    // PATH lookup by just running it.
    if Command::new("pandoc").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
    {
        return Some(PathBuf::from("pandoc"));
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(Path::new(&local).join("Pandoc").join("pandoc.exe"));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(Path::new(&program_files).join("Pandoc").join("pandoc.exe"));
        }
    }
    #[cfg(not(windows))]
    {
        for p in ["/usr/local/bin/pandoc", "/opt/homebrew/bin/pandoc", "/usr/bin/pandoc"] {
            candidates.push(PathBuf::from(p));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("md2docx: {msg}");
    eprintln!("Try 'md2docx --help'.");
    ExitCode::FAILURE
}

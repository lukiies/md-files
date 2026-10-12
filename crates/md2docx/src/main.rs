//! md2docx: convert a markdown file to a Word document.
//!
//! A cross-platform front-end for pandoc — the same conversion pipeline used
//! for the MSc report builds — with sane defaults: the input's folder is the
//! resource path (so relative images resolve), and pandoc is located even
//! when it is installed but not on PATH (the default Windows per-user
//! installer location, plus common Unix prefixes).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use md_core::itamd;

const USAGE: &str = "\
md2docx - convert Markdown to a Word document (via pandoc)

The .md file is treated as the editable SOURCE and the .docx as the compiled
deliverable: visible working notes (any blockquote opening with an emoji
marker, e.g. \"> \u{1f7e2} **[DRAFTED]** ...\"), HTML comments and legacy
[DRAFTED]/[EXPANSION NOTE] lines are stripped before conversion, so they
never reach the document. A \"> \u{1f9ed} **[TABLE OF CONTENTS]**\" marker
note turns into a pandoc table of contents at conversion time.

ITA-MD 1.0 conditional directives (docs/ITA-MD) are stripped too, with every
branch kept and a warning: such a file is a source, not a deliverable. Use
--itamd-profile to compile it for one configuration instead.

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
                        any TBC placeholder remains in the source, or while
                        it still holds uncompiled ITA-MD directives
  --itamd-profile FACTS Compile the ITA-MD directives for the configuration
                        in FACTS (a JSON object of facts, or a generator
                        manifest with a \"facts\" member) before converting;
                        any ITA-MD error stops the conversion
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
    let mut itamd_profile: Option<PathBuf> = None;

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
            "--itamd-profile" => match args.next() {
                Some(v) => itamd_profile = Some(PathBuf::from(v)),
                None => return fail("missing value for --itamd-profile"),
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

    let text = match std::fs::read_to_string(&input) {
        Ok(t) => t,
        Err(e) => return fail(&format!("cannot read {}: {e}", input.display())),
    };

    // --itamd-profile: compile the ITA-MD directives for one configuration
    // (fail closed). Without it, directives stay a source's text: they are
    // stripped with every branch kept, and the user is told so.
    let compiled = match &itamd_profile {
        Some(facts_path) => {
            let doc_dir = md_core::doc_base_dir(&input).unwrap_or_else(|| resource_dir.clone());
            let (vocabulary, problems) = itamd::vocabulary_for(&doc_dir);
            if !problems.is_empty() {
                return fail_itamd(&problems);
            }
            let facts = match itamd::load_facts(facts_path) {
                Ok(f) => f,
                Err(problems) => return fail_itamd(&problems),
            };
            match itamd::compile(&text, &facts, &itamd::CompileOptions { vocabulary: vocabulary.as_ref() }) {
                Ok(c) => Some(c.text),
                Err(problems) => return fail_itamd(&problems),
            }
        }
        None => None,
    };
    let uncompiled_directives = if compiled.is_none() { itamd::count_directives(&text) } else { 0 };
    if uncompiled_directives > 0 {
        eprintln!("md2docx: {}", itamd::WARNING_UNCOMPILED);
        if final_stage {
            eprintln!(
                "md2docx: REFUSING to convert as final: the source holds {uncompiled_directives} \
                 ITA-MD directive(s); compile it with --itamd-profile FACTS."
            );
            return ExitCode::FAILURE;
        }
    }

    // Source -> deliverable: strip working notes unless asked not to. The
    // cleaned text goes through a temp file; --resource-path keeps relative
    // images resolving against the real input's folder.
    let mut temp_input: Option<PathBuf> = None;
    let pandoc_input = if keep_notes {
        let verbatim = compiled.as_deref().unwrap_or(&text);
        if final_stage {
            if let Some(code) = refuse_final(verbatim) {
                return code;
            }
        }
        match &compiled {
            Some(c) => match write_temp(c) {
                Ok(temp) => {
                    temp_input = Some(temp.clone());
                    temp
                }
                Err(code) => return code,
            },
            None => input.clone(),
        }
    } else {
        let stripped = md_core::strip_source_notes(compiled.as_deref().unwrap_or(&text));
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
        match write_temp(&stripped.text) {
            Ok(temp) => {
                temp_input = Some(temp.clone());
                temp
            }
            Err(code) => return code,
        }
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

/// Write the text pandoc converts to a temp file (the real input's folder
/// stays the resource path).
fn write_temp(text: &str) -> Result<PathBuf, ExitCode> {
    let temp = std::env::temp_dir().join(format!("md2docx-{}.md", std::process::id()));
    match std::fs::write(&temp, text) {
        Ok(()) => Ok(temp),
        Err(e) => Err(fail(&format!("cannot write temp file {}: {e}", temp.display()))),
    }
}

/// Report ITA-MD problems (vocabulary, fact set, compilation) and fail.
fn fail_itamd(problems: &[itamd::Diagnostic]) -> ExitCode {
    for p in problems {
        eprintln!("md2docx: ITA-MD: {p}");
    }
    eprintln!("md2docx: ITA-MD: nothing written (the compiler fails closed)");
    ExitCode::FAILURE
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

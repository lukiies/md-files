//! md2html: convert a markdown file (or stdin) to HTML.
//!
//! The standalone successor of MDView's `--html` / `--body` output modes,
//! sharing the same engine (md-core). Cross-platform, no GUI dependencies.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use md_core::{itamd, render, wrap_html, AssetMode, PageOptions, RenderOptions};

const USAGE: &str = "\
md2html - convert Markdown to HTML (GFM, mermaid diagrams, syntax highlighting)

Usage:
  md2html [OPTIONS] [FILE]

Arguments:
  [FILE]           Markdown file to convert (reads from stdin if not provided)

Options:
  -o, --out FILE   Write output to FILE instead of stdout
  --save           Write output next to the input file (input name with
                   .html extension); requires a FILE input
  --body           Output the HTML body fragment only (no <html> wrapper)
  --embed          Self-contained page: inline mermaid/highlight bundles so the
                   file renders fully offline (default uses public CDNs)
  --dark           Dark color scheme (default: light)
  --title TEXT     Page <title> (default: the input filename)
  --strip-notes    Remove source-only working notes before converting
                   (emoji-blockquote notes like \"> \u{1f7e2} **[DRAFTED]** ...\",
                   HTML comments, ITA-MD directives with every branch kept)
                   - preview the document as a deliverable
  --itamd-profile FACTS
                   Compile the ITA-MD conditional directives for the
                   configuration in FACTS (a JSON object of facts, or a
                   generator manifest with a \"facts\" member); stubs are
                   dimmed. Any ITA-MD error stops the conversion.
  -h, --help       Show this help

A source with ITA-MD 1.0 directives (docs/ITA-MD) is shown annotated:
conditions as framed regions and chips, with a show/hide switch. The fact
vocabulary (itamd-slownik.json) is looked up from the input's folder upwards.

Examples:
  md2html README.md -o README.html
  md2html --embed --dark notes.md -o notes.html
  cat notes.md | md2html --body
";

fn main() -> ExitCode {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut body_only = false;
    let mut embed = false;
    let mut dark = false;
    let mut strip_notes = false;
    let mut save_next_to_input = false;
    let mut title: Option<String> = None;
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
            "--body" => body_only = true,
            "--embed" => embed = true,
            "--dark" => dark = true,
            "--strip-notes" => strip_notes = true,
            "--save" => save_next_to_input = true,
            "--title" => match args.next() {
                Some(v) => title = Some(v),
                None => return fail("missing value for --title"),
            },
            "--itamd-profile" => match args.next() {
                Some(v) => itamd_profile = Some(PathBuf::from(v)),
                None => return fail("missing value for --itamd-profile"),
            },
            _ if arg.starts_with('-') => {
                return fail(&format!("unknown option: {arg}"));
            }
            _ => {
                if input.is_some() {
                    return fail("more than one input file given");
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    // --save: derive the output path from the input file (context-menu use).
    if save_next_to_input {
        match (&input, &output) {
            (None, _) => return fail("--save needs a FILE input (not stdin)"),
            (Some(path), None) => output = Some(path.with_extension("html")),
            (Some(_), Some(_)) => {} // explicit -o wins
        }
    }

    // Read the source: file argument or stdin.
    let (markdown, base_dir, default_title) = match &input {
        Some(path) => {
            let text = match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => return fail(&format!("cannot read {}: {e}", path.display())),
            };
            let title = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Markdown".to_string());
            (text, md_core::doc_base_dir(path), title)
        }
        None => {
            let mut text = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut text) {
                return fail(&format!("cannot read stdin: {e}"));
            }
            (text, None, "Markdown".to_string())
        }
    };

    // ITA-MD: the project's fact vocabulary, found from the input's folder.
    let (vocabulary, vocabulary_problems) = match &base_dir {
        Some(dir) => itamd::vocabulary_for(dir),
        None => (None, Vec::new()),
    };

    // --itamd-profile: compile the conditional directives for one
    // configuration (fail closed: any ITA-MD error stops the conversion).
    let markdown = match &itamd_profile {
        Some(facts_path) => {
            if !vocabulary_problems.is_empty() {
                return fail_itamd(&vocabulary_problems);
            }
            let facts = match itamd::load_facts(facts_path) {
                Ok(f) => f,
                Err(problems) => return fail_itamd(&problems),
            };
            let options = itamd::CompileOptions { vocabulary: vocabulary.as_ref() };
            match itamd::compile(&markdown, &facts, &options) {
                Ok(compiled) => compiled.text,
                Err(problems) => return fail_itamd(&problems),
            }
        }
        None => markdown,
    };

    // --strip-notes: preview the deliverable — drop the source-only working
    // notes exactly the way md2docx does. Without it a source is shown as a
    // source: ITA-MD directives annotated, every branch visible.
    let markdown = if strip_notes {
        let stripped = md_core::strip_source_notes(&markdown);
        if stripped.itamd_directives > 0 {
            eprintln!("md2html: {}", itamd::WARNING_UNCOMPILED);
        }
        stripped.text
    } else if itamd_profile.is_none() {
        let annotated = itamd::annotate(
            &markdown,
            &itamd::AnnotateOptions {
                vocabulary: vocabulary.as_ref(),
                extra: vocabulary_problems,
                ..Default::default()
            },
        );
        let errors = annotated.diagnostics.iter().filter(|d| d.is_error()).count();
        if errors > 0 {
            eprintln!("md2html: ITA-MD: {errors} error(s) in the source (listed at the top of the page)");
        }
        annotated.markdown
    } else {
        markdown
    };

    // Image paths are left as authored (they resolve relative to the output's
    // location in a normal browser) — matching MDView's --html behavior.
    let mut body = render(&markdown, &RenderOptions::default()).body;
    if itamd_profile.is_some() {
        body = itamd::dim_stubs(&body);
    }

    // --embed: inline every local image as a data: URI so the page is fully
    // self-contained (openable from anywhere, no files needed next to it).
    if embed {
        if let Some(dir) = &base_dir {
            body = inline_local_images(&body, dir);
        }
    }

    let html = if body_only {
        body
    } else {
        wrap_html(
            &body,
            &PageOptions {
                dark_mode: dark,
                title: title.as_deref().unwrap_or(&default_title),
                assets: if embed { AssetMode::Embedded } else { AssetMode::Cdn },
                extra_js: "",
                itamd_hidden: false,
            },
        )
    };

    match &output {
        Some(path) => {
            if let Err(e) = std::fs::write(path, &html) {
                return fail(&format!("cannot write {}: {e}", path.display()));
            }
            eprintln!("wrote {} ({} bytes)", path.display(), html.len());
        }
        None => print!("{html}"),
    }
    ExitCode::SUCCESS
}

/// Replace `src="<local path>"` attributes with `data:` URIs for every local
/// image that exists on disk relative to `base_dir`. External URLs and
/// already-inlined images are left untouched.
fn inline_local_images(body: &str, base_dir: &std::path::Path) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(idx) = rest.find("src=\"") {
        let (before, after) = rest.split_at(idx + 5);
        out.push_str(before);
        let Some(end) = after.find('"') else {
            out.push_str(after);
            return out;
        };
        let url = &after[..end];
        rest = &after[end..];
        if url.is_empty() || url.starts_with('#') || url.starts_with("//") || md_core::has_uri_scheme(url) {
            out.push_str(url);
            continue;
        }
        let decoded = md_core::percent_decode(url);
        let rel = std::path::Path::new(&decoded);
        let abs = if rel.is_absolute() { rel.to_path_buf() } else { base_dir.join(rel) };
        let abs = md_core::normalize_path(&abs);
        match std::fs::read(&abs) {
            Ok(bytes) => {
                out.push_str(&format!(
                    "data:{};base64,{}",
                    md_core::mime_for(&abs),
                    md_core::base64_encode(&bytes)
                ));
            }
            Err(_) => out.push_str(url),
        }
    }
    out.push_str(rest);
    out
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("md2html: {msg}");
    eprintln!("Try 'md2html --help'.");
    ExitCode::FAILURE
}

/// Report ITA-MD problems (vocabulary, fact set, compilation) and fail.
fn fail_itamd(problems: &[itamd::Diagnostic]) -> ExitCode {
    for p in problems {
        eprintln!("md2html: ITA-MD: {p}");
    }
    eprintln!("md2html: ITA-MD: nothing written (the compiler fails closed)");
    ExitCode::FAILURE
}

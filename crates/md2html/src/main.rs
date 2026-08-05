//! md2html: convert a markdown file (or stdin) to HTML.
//!
//! The standalone successor of MDView's `--html` / `--body` output modes,
//! sharing the same engine (md-core). Cross-platform, no GUI dependencies.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use md_core::{render, wrap_html, AssetMode, PageOptions, RenderOptions};

const USAGE: &str = "\
md2html - convert Markdown to HTML (GFM, mermaid diagrams, syntax highlighting)

Usage:
  md2html [OPTIONS] [FILE]

Arguments:
  [FILE]           Markdown file to convert (reads from stdin if not provided)

Options:
  -o, --out FILE   Write output to FILE instead of stdout
  --body           Output the HTML body fragment only (no <html> wrapper)
  --embed          Self-contained page: inline mermaid/highlight bundles so the
                   file renders fully offline (default uses public CDNs)
  --dark           Dark color scheme (default: light)
  --title TEXT     Page <title> (default: the input filename)
  -h, --help       Show this help

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
    let mut title: Option<String> = None;

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
            "--title" => match args.next() {
                Some(v) => title = Some(v),
                None => return fail("missing value for --title"),
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

    // Image paths are left as authored (they resolve relative to the output's
    // location in a normal browser) — matching MDView's --html behavior.
    let mut body = render(&markdown, &RenderOptions::default()).body;

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

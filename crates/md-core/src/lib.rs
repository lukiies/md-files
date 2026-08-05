//! md-core: the shared markdown-to-HTML engine of the md-files tool suite.
//!
//! Ported from the MDView fork's renderer and generalized so the same engine
//! drives the `md2html` CLI converter and the cross-platform `mdread` viewer:
//!
//! * GitHub Flavored Markdown (tables, footnotes, strikethrough, task lists)
//! * ```mermaid fences become `<pre class="mermaid">` diagram containers
//! * fenced code blocks keep their `language-…` class for highlight.js
//! * local image references are resolved on disk and rewritten to a
//!   configurable document base URL (virtual host), widened to the deepest
//!   common ancestor folder when images live outside the document's folder;
//!   images with no common ancestor (another drive) are inlined as `data:` URIs
//! * local `.md` link destinations can be rewritten through a caller-supplied
//!   hook so a viewer can route them through its own navigation scheme
//! * pages are assembled by [`wrap_html`] with light/dark styling and the
//!   mermaid/highlight libraries pulled from a CDN, embedded inline, or served
//!   from a caller-controlled base URL — libraries are only included when the
//!   document actually needs them.

use std::path::{Component, Path, PathBuf};

use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// The bundled JS/CSS assets, embedded at compile time so every tool in the
/// suite can work fully offline.
pub mod assets {
    pub const MERMAID_JS: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/web/mermaid.min.js"));
    pub const HIGHLIGHT_JS: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/web/highlight.min.js"));
    pub const HLJS_CSS_LIGHT: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/web/hljs-github.css"));
    pub const HLJS_CSS_DARK: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/web/hljs-github-dark.css"));
}

/// Where a generated page loads mermaid.js / highlight.js from.
#[derive(Clone)]
pub enum AssetMode {
    /// Public CDNs — for HTML meant to be opened in a normal browser online.
    Cdn,
    /// The bundles are inlined into the page itself — a single self-contained
    /// file that renders offline (adds a few MB when diagrams/code are present).
    Embedded,
    /// Served from a caller-controlled base URL (no trailing slash), e.g. a
    /// WebView virtual host like `https://mdfiles.assets`.
    BaseUrl(String),
}

/// Everything [`render`] needs to know about the document's surroundings.
pub struct RenderOptions<'a> {
    /// Folder the document's relative paths resolve against. `None` leaves
    /// image and link destinations untouched (plain conversion for a browser).
    pub base_dir: Option<&'a Path>,
    /// Base URL (no trailing slash) local images are served from, e.g.
    /// `https://mdfiles.doc`. Only used when `base_dir` is `Some`.
    pub doc_url_base: Option<&'a str>,
    /// When set, a local link to an existing `.md`/`.markdown` file is passed
    /// to this hook (absolute path + optional `#fragment`) and replaced with
    /// the returned URL. Lets a viewer route markdown links through its own
    /// navigation scheme while external links stay untouched.
    #[allow(clippy::type_complexity)]
    pub md_link_rewrite: Option<&'a dyn Fn(&Path, Option<&str>) -> String>,
}

impl<'a> Default for RenderOptions<'a> {
    fn default() -> Self {
        Self { base_dir: None, doc_url_base: None, md_link_rewrite: None }
    }
}

/// A rendered document body plus the folder its file references live under.
pub struct Rendered {
    /// The HTML fragment (no `<html>` wrapper — see [`wrap_html`]).
    pub body: String,
    /// The folder the document virtual host must serve so every rewritten
    /// image URL resolves: the document's folder, widened to the deepest
    /// common ancestor of all referenced same-drive images. `None` when
    /// rendering without a `base_dir`.
    pub serve_root: Option<PathBuf>,
}

/// The pulldown-cmark options every conversion in the suite uses.
fn md_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options
}

/// Render markdown to an HTML fragment. Convenience wrapper over [`render`]
/// for callers that only need the body.
pub fn markdown_to_html(markdown: &str, base_dir: Option<&Path>) -> String {
    let doc_base = base_dir.map(|_| "https://mdfiles.doc");
    render(
        markdown,
        &RenderOptions { base_dir, doc_url_base: doc_base, md_link_rewrite: None },
    )
    .body
}

/// Render markdown to an HTML fragment according to `opts` (see
/// [`RenderOptions`] and [`Rendered`]).
pub fn render(markdown: &str, opts: &RenderOptions) -> Rendered {
    // Windows editors often save markdown with a UTF-8 BOM, which would
    // otherwise glue itself to the first heading marker and break parsing.
    let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let serve_root = opts.base_dir.map(|dir| serve_root_for(markdown, dir));

    let parser = Parser::new_ext(markdown, md_options());

    // Transform the event stream: ```mermaid fences become
    // `<pre class="mermaid">…</pre>` (for mermaid.js to render), while every
    // other fenced block is left untouched so pulldown emits
    // `<pre><code class="language-…">`, which highlight.js picks up. Image
    // destinations are rewritten so relative paths resolve to disk, and local
    // `.md` link destinations go through the caller's rewrite hook.
    let mut events: Vec<Event> = Vec::new();
    let mut in_mermaid = false;
    let mut mermaid_src = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) if is_mermaid(info) => {
                in_mermaid = true;
                mermaid_src.clear();
            }
            Event::Text(ref text) if in_mermaid => {
                mermaid_src.push_str(text);
            }
            Event::End(TagEnd::CodeBlock) if in_mermaid => {
                in_mermaid = false;
                // The source is HTML-escaped so the browser stores it as text
                // content verbatim (including literal `<br/>` in labels),
                // which is exactly what mermaid expects to parse.
                let block =
                    format!("<pre class=\"mermaid\">{}</pre>\n", escape_html(&mermaid_src));
                events.push(Event::Html(block.into()));
            }
            Event::Start(Tag::Image { link_type, dest_url, title, id }) => {
                let dest_url = match (opts.base_dir, serve_root.as_deref(), opts.doc_url_base) {
                    (Some(dir), Some(root), Some(base)) => {
                        rewrite_image_dest(&dest_url, dir, root, base).into()
                    }
                    _ => dest_url,
                };
                events.push(Event::Start(Tag::Image { link_type, dest_url, title, id }));
            }
            Event::Start(Tag::Link { link_type, dest_url, title, id }) => {
                let dest_url = match (opts.base_dir, opts.md_link_rewrite) {
                    (Some(dir), Some(hook)) => rewrite_md_link(&dest_url, dir, hook).into(),
                    _ => dest_url,
                };
                events.push(Event::Start(Tag::Link { link_type, dest_url, title, id }));
            }
            other => events.push(other),
        }
    }

    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    Rendered { body, serve_root }
}

/// The directory a document's relative paths resolve against: the (absolute,
/// lexically normalized) parent folder of `file_path`. Returns `None` if the
/// path has no parent or the working directory cannot be read.
pub fn doc_base_dir(file_path: &Path) -> Option<PathBuf> {
    let abs = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(file_path)
    };
    Some(normalize_path(abs.parent()?))
}

/// Compute the serve root for a document's content: starts at `base_dir` and
/// widens to the shared ancestor whenever an image resolves to an existing
/// file outside the current root. Images on a different drive (no shared
/// ancestor) never widen the root — they are inlined instead.
pub fn serve_root_for(markdown: &str, base_dir: &Path) -> PathBuf {
    let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let base = normalize_path(base_dir);
    let mut root = base.clone();

    for event in Parser::new_ext(markdown, md_options()) {
        if let Event::Start(Tag::Image { dest_url, .. }) = event {
            let dest = dest_url.as_ref();
            if dest.is_empty() || dest.starts_with('#') || dest.starts_with("//") || has_uri_scheme(dest) {
                continue;
            }
            let decoded = percent_decode(dest);
            let rel = Path::new(&decoded);
            let joined = if rel.is_absolute() { rel.to_path_buf() } else { base.join(rel) };
            let abs = normalize_path(&joined);
            if abs.is_file() && abs.strip_prefix(&root).is_err() {
                if let Some(shared) = shared_prefix(&root, &abs) {
                    root = shared;
                }
            }
        }
    }
    root
}

/// The longest common component prefix of two absolute paths, or `None` when
/// they share nothing usable as a folder (on Windows: different drives / UNC
/// shares; on Unix the filesystem root is always shared).
fn shared_prefix(a: &Path, b: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for (x, y) in a.components().zip(b.components()) {
        if x != y {
            break;
        }
        out.push(x.as_os_str());
    }
    let mut comps = out.components();
    match comps.next() {
        // Windows: need at least `<drive>\` to back a virtual-host mapping.
        Some(Component::Prefix(_)) => match comps.next() {
            Some(Component::RootDir) => Some(out),
            _ => None,
        },
        // Unix: `/` itself is a valid (if wide) serve root.
        Some(Component::RootDir) => Some(out),
        _ => None,
    }
}

/// Rewrite a markdown image destination so a WebView document can display it:
///
/// * External or already-absolute-scheme URLs (`http:`, `https:`, `data:`,
///   protocol-relative `//…`) and fragment-only refs are returned unchanged.
/// * A local path that resolves to an existing file under `serve_root` becomes
///   `<doc_url_base>/<rel>` served from disk by the document virtual host —
///   the HTML stays small no matter how large the images are.
/// * A local path outside `serve_root` (an image on another drive) is inlined
///   as a `data:` URI so it still shows.
/// * Anything that cannot be resolved to an existing file is returned
///   unchanged.
fn rewrite_image_dest(dest: &str, base_dir: &Path, serve_root: &Path, doc_url_base: &str) -> String {
    if dest.is_empty() || dest.starts_with('#') || dest.starts_with("//") || has_uri_scheme(dest) {
        return dest.to_string();
    }

    // Percent-decode so an encoded path (e.g. `figures/my%20image.png`) maps
    // to the real filename on disk.
    let decoded = percent_decode(dest);
    let rel = Path::new(&decoded);
    let joined = if rel.is_absolute() { rel.to_path_buf() } else { base_dir.join(rel) };
    let abs = normalize_path(&joined);

    if !abs.is_file() {
        return dest.to_string();
    }

    // Under the serve root: serve via the doc virtual host.
    if let Ok(relative) = abs.strip_prefix(serve_root) {
        let mut url = doc_url_base.to_string();
        for comp in relative.components() {
            if let Component::Normal(seg) = comp {
                url.push('/');
                url.push_str(&percent_encode(&seg.to_string_lossy()));
            }
        }
        return url;
    }

    // No shared ancestor with the document (another drive): inline the bytes
    // as a data URI.
    match std::fs::read(&abs) {
        Ok(bytes) => format!("data:{};base64,{}", mime_for(&abs), base64_encode(&bytes)),
        Err(_) => dest.to_string(),
    }
}

/// Rewrite a local `.md`/`.markdown` link destination through the caller's
/// hook. External URLs, fragments, and links to files that do not exist (or
/// are not markdown) are returned unchanged.
fn rewrite_md_link(
    dest: &str,
    base_dir: &Path,
    hook: &dyn Fn(&Path, Option<&str>) -> String,
) -> String {
    if dest.is_empty() || dest.starts_with('#') || dest.starts_with("//") || has_uri_scheme(dest) {
        return dest.to_string();
    }
    let (path_part, fragment) = match dest.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (dest, None),
    };
    let decoded = percent_decode(path_part);
    let ext_ok = {
        let lower = decoded.to_ascii_lowercase();
        lower.ends_with(".md") || lower.ends_with(".markdown")
    };
    if !ext_ok {
        return dest.to_string();
    }
    let rel = Path::new(&decoded);
    let joined = if rel.is_absolute() { rel.to_path_buf() } else { base_dir.join(rel) };
    let abs = normalize_path(&joined);
    if !abs.is_file() {
        return dest.to_string();
    }
    hook(&abs, fragment)
}

/// True if `s` begins with a URI scheme (`scheme:`), where scheme is at least
/// two characters. The two-char minimum keeps Windows drive-letter paths like
/// `C:\images\x.png` from being mistaken for a `c:` URL scheme.
pub fn has_uri_scheme(s: &str) -> bool {
    match s.find(':') {
        Some(idx) => {
            let scheme = &s[..idx];
            scheme.len() >= 2
                && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .take(idx)
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        }
        None => false,
    }
}

/// Lexically normalize a path (resolve `.`/`..`, no filesystem access).
pub fn normalize_path(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Normal(seg) => out.push(seg),
        }
    }
    out
}

/// Percent-encode a single URL path segment (unreserved chars pass through).
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Percent-decode a string (invalid escapes are left literal).
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Guess a MIME type from a file extension (falls back to a generic binary
/// type, which browsers still sniff for common formats). Covers the image
/// formats the viewer displays plus the text assets it serves.
pub fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") | Some("jfif") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("avif") => "image/avif",
        Some("apng") => "image/apng",
        Some("tif") | Some("tiff") => "image/tiff",
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("txt") | Some("md") | Some("markdown") => "text/plain; charset=utf-8",
        Some("pdf") => "application/pdf",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Standard base64 encoding (for `data:` URIs).
pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// True if a fenced code-block info string selects the mermaid language
/// (first whitespace-separated token, case-insensitive).
fn is_mermaid(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .map(|tok| tok.eq_ignore_ascii_case("mermaid"))
        .unwrap_or(false)
}

/// Minimal HTML-text escaping for embedding raw source as element content.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Options for assembling a complete HTML page around a rendered body.
pub struct PageOptions<'a> {
    /// Dark or light styling (colors, hljs theme, mermaid theme).
    pub dark_mode: bool,
    /// The page `<title>`; usually the document filename.
    pub title: &'a str,
    /// Where mermaid.js / highlight.js come from.
    pub assets: AssetMode,
    /// Extra JavaScript appended at the end of `<body>` (viewer glue: link
    /// interception, scroll persistence, keyboard shortcuts). Empty for plain
    /// CLI output.
    pub extra_js: &'a str,
}

impl<'a> Default for PageOptions<'a> {
    fn default() -> Self {
        Self { dark_mode: false, title: "Markdown", assets: AssetMode::Cdn, extra_js: "" }
    }
}

/// Wrap a rendered body in a complete standalone HTML document.
pub fn wrap_html(content: &str, opts: &PageOptions) -> String {
    let dark_mode = opts.dark_mode;
    let bg_color = if dark_mode { "#1e1e1e" } else { "#ffffff" };
    let text_color = if dark_mode { "#d4d4d4" } else { "#24292e" };
    let code_bg = if dark_mode { "#2d2d2d" } else { "#f6f8fa" };
    let link_color = if dark_mode { "#58a6ff" } else { "#0366d6" };
    let border_color = if dark_mode { "#444" } else { "#e1e4e8" };

    // Only pull in the heavy libraries when the document actually needs them.
    let needs_hljs = content.contains("<pre><code");
    let needs_mermaid = content.contains("class=\"mermaid\"");

    // highlight.js color theme, matched to the page's light/dark mode.
    let hljs_theme_css = if needs_hljs {
        if dark_mode { assets::HLJS_CSS_DARK } else { assets::HLJS_CSS_LIGHT }
    } else {
        ""
    };

    // Library <script> tags + a single initializer (runs once the DOM is
    // ready). Embedded mode inlines the bundles for a self-contained page.
    let mermaid_theme = if dark_mode { "dark" } else { "default" };
    let mut feature_scripts = String::new();
    match &opts.assets {
        AssetMode::Embedded => {
            if needs_hljs {
                feature_scripts.push_str("<script>");
                feature_scripts.push_str(assets::HIGHLIGHT_JS);
                feature_scripts.push_str("</script>\n");
            }
            if needs_mermaid {
                feature_scripts.push_str("<script>");
                feature_scripts.push_str(assets::MERMAID_JS);
                feature_scripts.push_str("</script>\n");
            }
        }
        AssetMode::Cdn => {
            if needs_hljs {
                feature_scripts.push_str("<script src=\"https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js\"></script>\n");
            }
            if needs_mermaid {
                feature_scripts.push_str("<script src=\"https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js\"></script>\n");
            }
        }
        AssetMode::BaseUrl(base) => {
            if needs_hljs {
                feature_scripts
                    .push_str(&format!("<script src=\"{base}/highlight.min.js\"></script>\n"));
            }
            if needs_mermaid {
                feature_scripts
                    .push_str(&format!("<script src=\"{base}/mermaid.min.js\"></script>\n"));
            }
        }
    }
    if needs_hljs || needs_mermaid {
        feature_scripts.push_str(&format!(
            r#"<script>
(function() {{
    function run() {{
        try {{ if (window.hljs) {{ hljs.configure({{ignoreUnescapedHTML: true}}); hljs.highlightAll(); }} }} catch (e) {{}}
        try {{
            if (window.mermaid) {{
                mermaid.initialize({{ startOnLoad: false, securityLevel: 'loose', theme: '{mermaid_theme}' }});
                mermaid.run({{ querySelector: '.mermaid' }});
            }}
        }} catch (e) {{}}
    }}
    if (document.readyState !== 'loading') run();
    else document.addEventListener('DOMContentLoaded', run);
}})();
</script>
"#
        ));
    }

    let title = escape_html(opts.title);
    let extra_js = opts.extra_js;

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    font-size: 14px;
    line-height: 1.6;
    padding: 20px;
    max-width: 900px;
    margin: 0 auto;
    background-color: {bg_color};
    color: {text_color};
}}
a {{ color: {link_color}; text-decoration: none; cursor: pointer; }}
a:hover {{ text-decoration: underline; }}
code {{
    background-color: {code_bg};
    padding: 0.2em 0.4em;
    border-radius: 3px;
    font-family: "Cascadia Code", "Fira Code", Consolas, monospace;
    font-size: 85%;
}}
pre {{
    background-color: transparent;
    padding: 0;
    overflow: auto;
    border-radius: 6px;
}}
pre code {{
    display: block;
    padding: 16px;
    background-color: {code_bg};
    border-radius: 6px;
    overflow-x: auto;
    font-size: 90%;
}}
.mermaid {{
    background-color: transparent;
    text-align: center;
    line-height: normal;
    margin: 1em 0;
}}
.mermaid svg {{ max-width: 100%; height: auto; }}
blockquote {{
    border-left: 4px solid {border_color};
    margin: 0;
    padding-left: 16px;
    color: {text_color};
    opacity: 0.8;
}}
table {{
    border-collapse: collapse;
    width: 100%;
}}
th, td {{
    border: 1px solid {border_color};
    padding: 8px 12px;
    text-align: left;
}}
th {{
    background-color: {code_bg};
}}
img {{
    max-width: 100%;
}}
h1, h2 {{
    border-bottom: 1px solid {border_color};
    padding-bottom: 0.3em;
}}
hr {{
    border: none;
    border-top: 1px solid {border_color};
}}
input[type="checkbox"] {{
    margin-right: 0.5em;
}}
{hljs_theme_css}
</style>
</head>
<body>
{content}
{feature_scripts}
{extra_js}
</body>
</html>"#
    )
}

/// Strip markdown down to readable plain text.
pub fn markdown_to_plain_text(markdown: &str) -> String {
    let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let options = Options::empty();
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::new();

    for event in parser {
        match event {
            Event::Text(text) => output.push_str(&text),
            Event::Code(code) => {
                output.push('`');
                output.push_str(&code);
                output.push('`');
            }
            Event::SoftBreak | Event::HardBreak => output.push('\n'),
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => output.push_str("\n\n"),
            Event::Start(Tag::Heading { .. }) => {}
            Event::End(TagEnd::Heading(_)) => output.push_str("\n\n"),
            Event::Start(Tag::CodeBlock(_)) => output.push_str("\n```\n"),
            Event::End(TagEnd::CodeBlock) => output.push_str("```\n\n"),
            Event::Start(Tag::List(_)) => {}
            Event::End(TagEnd::List(_)) => output.push('\n'),
            Event::Start(Tag::Item) => output.push_str("  - "),
            Event::End(TagEnd::Item) => output.push('\n'),
            Event::Start(Tag::BlockQuote(_)) => output.push_str("> "),
            Event::End(TagEnd::BlockQuote(_)) => output.push('\n'),
            _ => {}
        }
    }

    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let md = "# Hello\n\nThis is **bold** and *italic*.";
        let html = markdown_to_html(md, None);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = markdown_to_html(md, None);
        // Passed through to pulldown so highlight.js can colorise it.
        assert!(html.contains("<pre><code class=\"language-rust\">"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_mermaid_block_becomes_diagram() {
        let md = "```mermaid\nflowchart TD\n  A[\"Start\"] --> B\n```";
        let html = markdown_to_html(md, None);
        assert!(html.contains("<pre class=\"mermaid\">"));
        assert!(!html.contains("language-mermaid"));
        assert!(html.contains("flowchart TD"));
        assert!(html.contains("&quot;Start&quot;"));
    }

    #[test]
    fn test_mermaid_escapes_html_in_labels() {
        let md = "```mermaid\nflowchart TD\n  Q1{\"Czy?<br/>tak\"}\n```";
        let html = markdown_to_html(md, None);
        assert!(html.contains("&lt;br/&gt;"));
    }

    #[test]
    fn test_wrap_includes_libraries_when_needed() {
        let body = markdown_to_html(
            "```mermaid\nflowchart TD\nA-->B\n```\n\n```rust\nlet x = 1;\n```",
            None,
        );
        let page = wrap_html(&body, &PageOptions { dark_mode: true, ..Default::default() });
        assert!(page.contains("mermaid.min.js"));
        assert!(page.contains("highlight.min.js"));
        assert!(page.contains("mermaid.run"));
    }

    #[test]
    fn test_wrap_omits_libraries_when_unneeded() {
        let body = markdown_to_html("# Just a heading\n\nSome text.", None);
        let page = wrap_html(&body, &PageOptions::default());
        assert!(!page.contains("mermaid.min.js"));
        assert!(!page.contains("highlight.min.js"));
    }

    #[test]
    fn test_wrap_embedded_inlines_bundles() {
        let body = markdown_to_html("```rust\nlet x = 1;\n```", None);
        let page = wrap_html(
            &body,
            &PageOptions { assets: AssetMode::Embedded, ..Default::default() },
        );
        // No external script URLs; the bundle text itself is present.
        assert!(!page.contains("<script src="));
        assert!(page.contains("hljs"));
    }

    #[test]
    fn test_images_untouched_without_base_dir() {
        let html = markdown_to_html("![Fig](figures/figure-2-1.png)", None);
        assert!(html.contains("src=\"figures/figure-2-1.png\""));
    }

    #[test]
    fn test_relative_image_uses_doc_virtual_host() {
        // Write a real file so the on-disk existence check passes.
        let dir = std::env::temp_dir().join("mdfiles_img_test_relative");
        let figures = dir.join("figures");
        std::fs::create_dir_all(&figures).unwrap();
        std::fs::write(figures.join("figure-2-1.png"), b"\x89PNG\r\n\x1a\n").unwrap();

        let html = markdown_to_html("![Fig 2.1](figures/figure-2-1.png)", Some(&dir));
        assert!(
            html.contains("src=\"https://mdfiles.doc/figures/figure-2-1.png\""),
            "got: {html}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_missing_image_left_unchanged() {
        let dir = std::env::temp_dir().join("mdfiles_img_test_missing");
        std::fs::create_dir_all(&dir).unwrap();
        let html = markdown_to_html("![x](figures/nope.png)", Some(&dir));
        assert!(html.contains("src=\"figures/nope.png\""), "got: {html}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_external_image_url_preserved() {
        let dir = std::env::temp_dir();
        let html = markdown_to_html("![x](https://example.com/a.png)", Some(&dir));
        assert!(html.contains("src=\"https://example.com/a.png\""));
        let data = markdown_to_html("![x](data:image/png;base64,AAAA)", Some(&dir));
        assert!(data.contains("src=\"data:image/png;base64,AAAA\""));
    }

    #[test]
    fn test_percent_encoded_relative_image_resolves() {
        let dir = std::env::temp_dir().join("mdfiles_img_test_encoded");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("my image.png"), b"\x89PNG\r\n\x1a\n").unwrap();
        let html = markdown_to_html("![x](my%20image.png)", Some(&dir));
        assert!(
            html.contains("src=\"https://mdfiles.doc/my%20image.png\""),
            "got: {html}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_md_link_rewrite_hook() {
        let dir = std::env::temp_dir().join("mdfiles_link_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("other.md"), "# other").unwrap();

        let hook = |abs: &Path, frag: Option<&str>| {
            let mut url = format!("app://page/{}", percent_encode(&abs.to_string_lossy()));
            if let Some(f) = frag {
                url.push('#');
                url.push_str(f);
            }
            url
        };
        let rendered = render(
            "[go](other.md#top) and [ext](https://example.com/x.md)",
            &RenderOptions {
                base_dir: Some(&dir),
                doc_url_base: Some("https://mdfiles.doc"),
                md_link_rewrite: Some(&hook),
            },
        );
        assert!(rendered.body.contains("app://page/"), "got: {}", rendered.body);
        assert!(rendered.body.contains("#top"), "got: {}", rendered.body);
        // External .md URL untouched.
        assert!(rendered.body.contains("https://example.com/x.md"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_utf8_bom_does_not_break_first_heading() {
        let md = "\u{feff}# Title\n\ntext";
        let html = markdown_to_html(md, None);
        assert!(html.contains("<h1>Title</h1>"), "got: {html}");
    }

    #[test]
    fn test_has_uri_scheme_vs_drive_letter() {
        assert!(has_uri_scheme("https://x"));
        assert!(has_uri_scheme("data:image/png;base64,AA"));
        assert!(!has_uri_scheme("C:\\images\\x.png")); // Windows drive, not a scheme
        assert!(!has_uri_scheme("figures/x.png"));
    }

    #[test]
    fn test_base64_encode_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}

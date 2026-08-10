//! mdread: a fast, cross-platform Markdown reader.
//!
//! The cross-platform successor of the MDView fork's GUI viewer, built on
//! wry + tao (WebView2 on Windows, WebKitGTK on Linux, WKWebView on macOS).
//!
//! Architecture: every page the WebView shows is served on demand through a
//! custom `mdfiles` protocol. A page URL encodes the absolute path of the
//! markdown file it renders, so
//!
//! * there is no HTML size limit (nothing is pushed as a string — the MDView
//!   2 MB `NavigateToString` problem cannot exist here),
//! * links between markdown files are plain navigations — the WebView's own
//!   history drives Back/Forward across files and folders,
//! * live reload is just `location.reload()` — the handler re-reads the file,
//! * images are served straight from disk with correct MIME types, however
//!   many and however large they are.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::http::{header::CONTENT_TYPE, Request, Response};
use wry::WebViewBuilder;
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;

use md_core::{percent_decode, percent_encode};

/// Requests dispatched from the IPC/protocol threads to the event loop.
#[derive(Debug)]
enum UserEvent {
    /// A page finished loading: apply the persisted zoom factor.
    PageLoaded,
    /// The document on screen changed on disk: reload it.
    ReloadDocument,
    /// Show the file-open dialog and navigate to the chosen file.
    OpenDialog,
    /// Open a non-markdown / external URL with the system handler.
    OpenExternal(String),
    /// Update the window title to the served document's filename.
    SetTitle(String),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Close,
}

/// The URL prefix the custom protocol is reachable under. wry maps custom
/// protocols differently per platform.
fn proto_base() -> &'static str {
    if cfg!(any(windows, target_os = "android")) {
        "https://mdfiles.localhost"
    } else {
        "mdfiles://localhost"
    }
}

/// The viewer URL that renders a given markdown file (+ optional fragment).
fn page_url(path: &Path, fragment: Option<&str>) -> String {
    let mut url = format!(
        "{}/page?p={}",
        proto_base(),
        percent_encode(&path.to_string_lossy())
    );
    if let Some(frag) = fragment {
        url.push('#');
        url.push_str(frag);
    }
    url
}

const USAGE: &str = "\
mdread - fast cross-platform Markdown reader

Usage:
  mdread [OPTIONS] [FILE]

Arguments:
  [FILE]      Markdown file to open (a file dialog is shown if omitted)

Options:
  --dark      Force dark mode
  --light     Force light mode (default: follow the system setting)
  -h, --help  Show this help

Keys:
  ESC                 close
  Ctrl+O              open file
  Ctrl+scroll / +/-   zoom (persisted)
  Ctrl+0              reset zoom
  Mouse Back/Forward  navigate history across linked .md files
";

fn main() {
    let mut file_arg: Option<PathBuf> = None;
    let mut force_dark: Option<bool> = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return;
            }
            "--dark" => force_dark = Some(true),
            "--light" => force_dark = Some(false),
            _ if arg.starts_with('-') => {
                eprintln!("mdread: unknown option: {arg}\nTry 'mdread --help'.");
                std::process::exit(2);
            }
            _ => file_arg = Some(PathBuf::from(arg)),
        }
    }

    let dark_mode = force_dark.unwrap_or_else(|| {
        matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
    });

    // MDREAD_SMOKE=1: exit 0 as soon as the first page reports DOMContentLoaded
    // through IPC (proof the protocol handler served it and the WebView
    // rendered), or 3 on timeout. Lets tests and scripts verify the full
    // window → protocol → render pipeline headlessly-ish (a window flashes).
    let smoke_test = std::env::var_os("MDREAD_SMOKE").is_some();
    if smoke_test {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(20));
            eprintln!("mdread: smoke test timed out waiting for page load");
            std::process::exit(3);
        });
    }

    // Resolve the file to open: CLI argument or file dialog.
    let initial = match file_arg {
        Some(p) => match std::fs::canonicalize(&p) {
            Ok(abs) => abs,
            Err(e) => {
                eprintln!("mdread: cannot open {}: {e}", p.display());
                std::process::exit(1);
            }
        },
        None => match pick_markdown_file() {
            Some(p) => p,
            None => return, // dialog cancelled
        },
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let title = format!(
        "{} - mdread",
        initial.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    );
    let window = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(LogicalSize::new(1000.0, 800.0))
        .build(&event_loop)
        .expect("failed to create window");

    // The document currently on screen, shared with the protocol handler
    // (which updates it on every page render) and the live-reload watcher.
    let current_doc: Arc<Mutex<PathBuf>> = Arc::new(Mutex::new(initial.clone()));

    // ---- custom protocol: pages, document files, bundled assets ------------
    let handler_doc = current_doc.clone();
    let handler_proxy = proxy.clone();
    let protocol = move |_id: wry::WebViewId, request: Request<Vec<u8>>| -> Response<Cow<'static, [u8]>> {
        #[cfg(debug_assertions)]
        eprintln!("[mdread] protocol request: {}", request.uri());
        serve(&request, dark_mode, &handler_doc, &handler_proxy)
    };

    // ---- IPC: keyboard/zoom/link glue from the injected script -------------
    let ipc_proxy = proxy.clone();
    let ipc = move |request: Request<String>| {
        let msg = request.body().as_str();
        let event = if let Some(url) = msg.strip_prefix("open-external:") {
            Some(UserEvent::OpenExternal(url.to_string()))
        } else {
            match msg {
                "loaded" => Some(UserEvent::PageLoaded),
                "open-dialog" => Some(UserEvent::OpenDialog),
                "zoom-in" => Some(UserEvent::ZoomIn),
                "zoom-out" => Some(UserEvent::ZoomOut),
                "zoom-reset" => Some(UserEvent::ZoomReset),
                "close" => Some(UserEvent::Close),
                _ => None,
            }
        };
        if let Some(event) = event {
            let _ = ipc_proxy.send_event(event);
        }
    };

    #[cfg(debug_assertions)]
    eprintln!("[mdread] initial={} url={}", initial.display(), page_url(&initial, None));

    let init_script = GLUE_JS.replace("__PROTO_BASE__", proto_base());

    let builder = WebViewBuilder::new()
        .with_url(page_url(&initial, None))
        .with_custom_protocol("mdfiles".to_string(), protocol)
        .with_initialization_script(&init_script)
        .with_ipc_handler(ipc);

    // On Windows, wry maps custom protocols to `<http|https>://mdfiles.localhost`
    // and intercepts only that prefix — and it defaults to http. Every URL this
    // viewer builds uses https (see proto_base), so the two MUST stay in sync:
    // without this call the interception filter never matches and every page
    // fails with ERR_CONNECTION_REFUSED.
    #[cfg(windows)]
    let builder = builder.with_https_scheme(true);

    #[cfg(any(windows, target_os = "macos"))]
    let webview = builder.build(&window).expect("failed to create webview");
    #[cfg(not(any(windows, target_os = "macos")))]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("no default vbox");
        builder.build_gtk(vbox).expect("failed to create webview")
    };

    // ---- live reload: poll the current document's timestamp/size ----------
    // Only metadata is polled — the file is never held open, so editors stay
    // free to write, rename, or delete it. A transient failed stat (save in
    // progress) is simply retried on the next tick.
    {
        let watch_doc = current_doc.clone();
        let watch_proxy = proxy.clone();
        std::thread::spawn(move || {
            let mut last: Option<(PathBuf, SystemTime, u64)> = None;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let path = watch_doc.lock().unwrap().clone();
                let Ok(meta) = std::fs::metadata(&path) else { continue };
                let Ok(modified) = meta.modified() else { continue };
                let stamp = (path.clone(), modified, meta.len());
                match &last {
                    Some(prev) if prev.0 == stamp.0 && (prev.1 != stamp.1 || prev.2 != stamp.2) => {
                        last = Some(stamp);
                        let _ = watch_proxy.send_event(UserEvent::ReloadDocument);
                    }
                    _ => last = Some(stamp),
                }
            }
        });
    }

    let mut zoom = load_zoom();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(user) => match user {
                UserEvent::PageLoaded => {
                    if smoke_test {
                        eprintln!("mdread: smoke test OK (page loaded)");
                        std::process::exit(0);
                    }
                    if (zoom - 1.0).abs() > f64::EPSILON {
                        let _ = webview.zoom(zoom);
                    }
                }
                UserEvent::ReloadDocument => {
                    let _ = webview.evaluate_script("location.reload()");
                }
                UserEvent::OpenDialog => {
                    if let Some(path) = pick_markdown_file() {
                        let _ = webview.load_url(&page_url(&path, None));
                    }
                }
                UserEvent::OpenExternal(url) => {
                    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
                        let _ = open::that_detached(&url);
                    }
                }
                UserEvent::SetTitle(name) => {
                    window.set_title(&format!("{name} - mdread"));
                }
                UserEvent::ZoomIn => {
                    zoom = (zoom * 1.1).min(5.0);
                    let _ = webview.zoom(zoom);
                    save_zoom(zoom);
                }
                UserEvent::ZoomOut => {
                    zoom = (zoom / 1.1).max(0.25);
                    let _ = webview.zoom(zoom);
                    save_zoom(zoom);
                }
                UserEvent::ZoomReset => {
                    zoom = 1.0;
                    let _ = webview.zoom(zoom);
                    save_zoom(zoom);
                }
                UserEvent::Close => {
                    *control_flow = ControlFlow::Exit;
                }
            },
            _ => {}
        }
    });
}

/// Serve one request of the `mdfiles` protocol.
///
/// Routes:
/// * `/page?p=<encoded abs path>` — render that markdown file as a full page
/// * `/assets/<bundle>`           — embedded mermaid/highlight bundles
/// * `/root/<encoded root>/<rel…>`— a file on disk under that serve root
///   (image references are rewritten to this form at render time, so every
///   page carries its own serve root and history navigation across folders
///   keeps working)
fn serve(
    request: &Request<Vec<u8>>,
    dark_mode: bool,
    current_doc: &Arc<Mutex<PathBuf>>,
    proxy: &tao::event_loop::EventLoopProxy<UserEvent>,
) -> Response<Cow<'static, [u8]>> {
    let uri = request.uri();
    let path = uri.path();

    if path == "/page" {
        let Some(doc) = uri.query().and_then(query_path) else {
            return not_found("missing p= query parameter");
        };
        return match render_page(&doc, dark_mode) {
            Ok(html) => {
                *current_doc.lock().unwrap() = doc.clone();
                let name = doc
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let _ = proxy.send_event(UserEvent::SetTitle(name));
                respond(html.into_bytes(), "text/html; charset=utf-8")
            }
            Err(e) => {
                let msg = format!(
                    "<!DOCTYPE html><meta charset=\"utf-8\"><h2>Cannot open document</h2><p>{}</p><p>{}</p>",
                    md_core::escape_html(&doc.to_string_lossy()),
                    md_core::escape_html(&e.to_string())
                );
                respond(msg.into_bytes(), "text/html; charset=utf-8")
            }
        };
    }

    if let Some(asset) = path.strip_prefix("/assets/") {
        return match asset {
            "mermaid.min.js" => respond(
                md_core::assets::MERMAID_JS.as_bytes().to_vec(),
                "text/javascript; charset=utf-8",
            ),
            "highlight.min.js" => respond(
                md_core::assets::HIGHLIGHT_JS.as_bytes().to_vec(),
                "text/javascript; charset=utf-8",
            ),
            _ => not_found("unknown asset"),
        };
    }

    if let Some(rest) = path.strip_prefix("/root/") {
        return match resolve_root_file(rest) {
            Some(file) => match std::fs::read(&file) {
                Ok(bytes) => respond(bytes, md_core::mime_for(&file)),
                Err(_) => not_found("file not found"),
            },
            None => not_found("bad path segment"),
        };
    }

    not_found("unknown route")
}

/// Resolve a `/root/<encoded abs root>/<rel segments…>` URL path (with the
/// `/root/` prefix already stripped) to the on-disk file it addresses. The
/// first segment is the percent-encoded absolute serve root; the remaining
/// segments are the file's path below it. Returns `None` for traversal or
/// separator-carrying segments — the root already encodes the full allowed
/// prefix.
fn resolve_root_file(rest: &str) -> Option<PathBuf> {
    let mut segments = rest.split('/');
    let root_enc = segments.next()?;
    let mut file = PathBuf::from(percent_decode(root_enc));
    for seg in segments {
        let decoded = percent_decode(seg);
        if decoded == ".." || decoded.contains('\\') || decoded.contains('/') {
            return None;
        }
        file.push(decoded);
    }
    Some(file)
}

/// Extract and decode the `p=` parameter of a page URL's query string.
fn query_path(query: &str) -> Option<PathBuf> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "p").then(|| PathBuf::from(percent_decode(v)))
    })
}

/// Render one markdown file into a complete HTML page.
fn render_page(doc: &Path, dark_mode: bool) -> std::io::Result<String> {
    let markdown = std::fs::read_to_string(doc)?;
    let base_dir = md_core::doc_base_dir(doc)
        .ok_or_else(|| std::io::Error::other("document has no parent folder"))?;

    // Image URLs carry their serve root: /root/<encoded root>/<rel…>.
    let serve_root = md_core::serve_root_for(&markdown, &base_dir);
    let doc_url_base = format!(
        "{}/root/{}",
        proto_base(),
        percent_encode(&serve_root.to_string_lossy())
    );

    // Local .md links become /page URLs so clicking them is a plain
    // navigation and the WebView's history drives Back/Forward.
    let link_hook = |abs: &Path, frag: Option<&str>| page_url(abs, frag);

    let rendered = md_core::render(
        &markdown,
        &md_core::RenderOptions {
            base_dir: Some(&base_dir),
            doc_url_base: Some(&doc_url_base),
            md_link_rewrite: Some(&link_hook),
        },
    );

    let title = doc
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Markdown".to_string());

    Ok(md_core::wrap_html(
        &rendered.body,
        &md_core::PageOptions {
            dark_mode,
            title: &title,
            assets: md_core::AssetMode::BaseUrl(format!("{}/assets", proto_base())),
            extra_js: "",
        },
    ))
}

fn respond(bytes: Vec<u8>, mime: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, mime)
        .body(Cow::Owned(bytes))
        .unwrap()
}

fn not_found(msg: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(404)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Cow::Owned(msg.as_bytes().to_vec()))
        .unwrap()
}

fn pick_markdown_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown"])
        .pick_file()
}

// ---- persisted zoom ---------------------------------------------------------

fn config_file() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("md-files").join("mdread.conf"))
}

fn load_zoom() -> f64 {
    let Some(path) = config_file() else { return 1.0 };
    let Ok(text) = std::fs::read_to_string(path) else { return 1.0 };
    parse_zoom(&text)
}

/// Extract the persisted zoom factor from the config file's text; anything
/// missing, unparsable, or outside the UI's zoom bounds falls back to 1.0.
fn parse_zoom(text: &str) -> f64 {
    text.lines()
        .find_map(|l| l.strip_prefix("zoom=")?.trim().parse::<f64>().ok())
        .filter(|z| (0.25..=5.0).contains(z))
        .unwrap_or(1.0)
}

fn save_zoom(zoom: f64) {
    let Some(path) = config_file() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, format!("zoom={zoom:.3}\n"));
}

// ---- the glue script injected into every page ------------------------------
// Runs before the page's own scripts on every navigation. Handles: scroll
// persistence across reloads (sessionStorage), external-link interception,
// and the keyboard/zoom shortcuts.
const GLUE_JS: &str = r#"
(function(){
  var BASE = "__PROTO_BASE__";
  function skey(){ return "mdread-scroll:" + location.href.split('#')[0]; }
  var scrollTimer = null;
  window.addEventListener('scroll', function(){
    if (scrollTimer) return;
    scrollTimer = setTimeout(function(){
      scrollTimer = null;
      try { sessionStorage.setItem(skey(), String(Math.round(window.scrollY || 0))); } catch (e) {}
    }, 100);
  }, {passive:true});
  window.addEventListener('DOMContentLoaded', function(){
    try {
      var y = sessionStorage.getItem(skey());
      if (y && !location.hash) window.scrollTo(0, parseInt(y, 10));
    } catch (e) {}
    try { window.ipc.postMessage('loaded'); } catch (e) {}
  });
  document.addEventListener('click', function(e){
    var link = e.target && e.target.closest ? e.target.closest('a') : null;
    if (!link) return;
    var href = link.getAttribute('href') || '';
    if (!href || href.charAt(0) === '#') return;
    var abs = link.href || href;
    if (abs.indexOf(BASE) === 0) return; // internal: native navigation + history
    e.preventDefault();
    window.ipc.postMessage('open-external:' + abs);
  });
  document.addEventListener('keydown', function(e){
    if (e.key === 'Escape') { window.ipc.postMessage('close'); return; }
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key === 'o' || e.key === 'O') { e.preventDefault(); window.ipc.postMessage('open-dialog'); }
    else if (e.key === '0') { e.preventDefault(); window.ipc.postMessage('zoom-reset'); }
    else if (e.key === '+' || e.key === '=') { e.preventDefault(); window.ipc.postMessage('zoom-in'); }
    else if (e.key === '-') { e.preventDefault(); window.ipc.postMessage('zoom-out'); }
  });
  window.addEventListener('wheel', function(e){
    if (!e.ctrlKey) return;
    e.preventDefault();
    window.ipc.postMessage(e.deltaY < 0 ? 'zoom-in' : 'zoom-out');
  }, {passive:false});
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_url_encodes_path_and_keeps_fragment() {
        let url = page_url(Path::new("C:\\docs\\my notes.md"), Some("top"));
        assert!(url.starts_with(proto_base()), "got: {url}");
        assert!(url.contains("/page?p="), "got: {url}");
        // Space and backslash are percent-encoded, fragment survives verbatim.
        assert!(url.contains("%20"), "got: {url}");
        assert!(!url[url.find("p=").unwrap()..].contains(' '), "got: {url}");
        assert!(url.ends_with("#top"), "got: {url}");
    }

    #[test]
    fn test_query_path_roundtrips_through_page_url() {
        let original = Path::new("C:\\docs\\my notes.md");
        let url = page_url(original, None);
        let query = url.split('?').nth(1).unwrap();
        assert_eq!(query_path(query), Some(original.to_path_buf()));
    }

    #[test]
    fn test_query_path_picks_p_among_other_params() {
        assert_eq!(query_path("a=1&p=x.md&b=2"), Some(PathBuf::from("x.md")));
        assert_eq!(query_path("a=1&b=2"), None);
        assert_eq!(query_path(""), None);
    }

    #[test]
    fn test_resolve_root_file_joins_segments() {
        let root_enc = percent_encode("C:\\serve root");
        let resolved = resolve_root_file(&format!("{root_enc}/figures/fig%201.png")).unwrap();
        assert_eq!(
            resolved,
            Path::new("C:\\serve root").join("figures").join("fig 1.png")
        );
    }

    #[test]
    fn test_resolve_root_file_rejects_traversal() {
        let root_enc = percent_encode("C:\\serve");
        // Literal, encoded, and separator-smuggling traversal all rejected.
        assert!(resolve_root_file(&format!("{root_enc}/../secret.txt")).is_none());
        assert!(resolve_root_file(&format!("{root_enc}/%2E%2E/secret.txt")).is_none());
        assert!(resolve_root_file(&format!("{root_enc}/a%5Cb.txt")).is_none()); // encoded '\'
        assert!(resolve_root_file(&format!("{root_enc}/a%2Fb.txt")).is_none()); // encoded '/'
    }

    #[test]
    fn test_parse_zoom_bounds() {
        assert_eq!(parse_zoom("zoom=1.500\n"), 1.5);
        assert_eq!(parse_zoom("zoom=99"), 1.0); // out of range -> default
        assert_eq!(parse_zoom("zoom=0.1"), 1.0); // below minimum -> default
        assert_eq!(parse_zoom("garbage"), 1.0);
        assert_eq!(parse_zoom(""), 1.0);
    }
}

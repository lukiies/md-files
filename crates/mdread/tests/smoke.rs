//! End-to-end render smoke test: launches the real mdread binary in
//! MDREAD_SMOKE mode, which exits 0 only once the page's DOMContentLoaded
//! reaches the app through IPC — i.e. the custom protocol actually served
//! the document and the WebView rendered it. This is the test that catches
//! protocol-registration bugs (e.g. the wry http-vs-https scheme mismatch
//! that made every page fail with ERR_CONNECTION_REFUSED).
//!
//! Windows-only: it needs a desktop session and the WebView2 runtime. A
//! window flashes briefly while the test runs.

#![cfg(windows)]

use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

/// The smoke tests start real mdread processes one at a time: WebView2
/// instances that start at the same moment can fail to share their user
/// data folder (E_INVALIDARG), which would make the tests flaky.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn first_page_actually_renders() {
    let _guard = serial();
    let demo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("demo")
        .join("manual-test.md");
    assert!(demo.is_file(), "demo fixture missing: {}", demo.display());

    let out = Command::new(env!("CARGO_BIN_EXE_mdread"))
        .arg(&demo)
        .env("MDREAD_SMOKE", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "mdread did not render the page (exit {:?}); stderr:\n{stderr}",
        out.status.code()
    );
    assert!(stderr.contains("smoke test OK"), "stderr:\n{stderr}");
}

/// The ITA-MD example of the handbook (docs/ITA-MD/przyklady).
fn itamd_example(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("ITA-MD")
        .join("przyklady")
        .join(name)
}

/// Run the smoke mode and return the page's load report, e.g.
/// `itamd-chips=12,itamd-stubs=0` (counted in the rendered DOM).
fn smoke_report(args: &[&std::ffi::OsStr]) -> (usize, usize) {
    let _guard = serial();
    let out =Command::new(env!("CARGO_BIN_EXE_mdread")).args(args).env("MDREAD_SMOKE", "1").output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "exit {:?}; stderr:\n{stderr}", out.status.code());
    let number = |key: &str| -> usize {
        let at = stderr.find(key).unwrap_or_else(|| panic!("no {key} in stderr:\n{stderr}"));
        stderr[at + key.len()..].chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap()
    };
    (number("itamd-chips="), number("itamd-stubs="))
}

#[test]
fn itamd_source_renders_its_directives_as_chips() {
    let source = itamd_example("cennik-fragment.md");
    let (chips, stubs) = smoke_report(&[source.as_os_str()]);
    // 4 blocks, 1 inline group (2 chips: JEŻELI and the closing bracket),
    // 7 rows, 1 table directive.
    assert_eq!(chips, 4 + 2 + 7 + 1, "ITA-MD chips rendered");
    assert_eq!(stubs, 0, "an annotated source has no stubs");
}

#[test]
fn itamd_profile_preview_renders_the_compiled_text() {
    let source = itamd_example("cennik-fragment.md");
    let facts = itamd_example("fakty-T3.json");
    let (chips, stubs) = smoke_report(&["--itamd-profile".as_ref(), facts.as_os_str(), source.as_os_str()]);
    assert_eq!(chips, 0, "a compiled preview shows no directives");
    // T3: lit. c and d, E3, and the E5.2/E5.3 rows are stubs.
    assert_eq!(stubs, 5, "dimmed stubs rendered");
}

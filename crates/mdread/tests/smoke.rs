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

#[test]
fn first_page_actually_renders() {
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

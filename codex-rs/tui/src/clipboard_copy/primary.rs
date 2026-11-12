//! Local Linux PRIMARY selection, using X11 or a capable Wayland compositor.
//! Never forward PRIMARY over SSH or tmux, or target the WSL host clipboard.

#[cfg(target_os = "linux")]
use super::ClipboardLease;
use super::CopyOutcome;
use std::time::Instant;

#[cfg(target_os = "linux")]
pub(crate) fn available() -> bool {
    if super::is_ssh_session()
        || super::is_tmux_session()
        || super::is_wsl_session()
        || crate::tui::detect_vscode_terminal() == crate::tui::VscodeDetection::VsCode
    {
        return false;
    }

    display_available(
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var_os("DISPLAY").is_some(),
        || {
            // Probe once without reading or replacing either clipboard. Arboard uses
            // this same Wayland backend, so unsupported compositors stay disabled.
            static SUPPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            *SUPPORTED.get_or_init(|| {
                wl_clipboard_rs::utils::is_primary_selection_supported().unwrap_or_else(|err| {
                    tracing::debug!(%err, "Wayland PRIMARY selection unavailable");
                    false
                })
            })
        },
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn available() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn display_available(wayland: bool, x11: bool, wayland_primary: impl FnOnce() -> bool) -> bool {
    if wayland { wayland_primary() } else { x11 }
}

#[cfg(target_os = "linux")]
pub(super) fn copy(
    text: &str,
    begin_delivery: impl FnOnce() -> Result<(), String>,
) -> Result<CopyOutcome, String> {
    use arboard::SetExtLinux;
    let mut clipboard = arboard::Clipboard::new().map_err(|err| err.to_string())?;
    begin_delivery()?;
    clipboard
        .set()
        .clipboard(arboard::LinuxClipboardKind::Primary)
        .text(text)
        .map_err(|err| format!("PRIMARY selection: {err}"))?;
    Ok(CopyOutcome::Copied(Some(ClipboardLease::native_linux(
        clipboard,
    ))))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn copy(
    _text: &str,
    _begin_delivery: impl FnOnce() -> Result<(), String>,
) -> Result<CopyOutcome, String> {
    Err("PRIMARY selection unavailable".into())
}

#[cfg(target_os = "linux")]
pub(super) fn read(deadline: Instant) -> Result<String, String> {
    use arboard::GetExtLinux;
    crate::clipboard_paste::text::native_result(
        arboard::Clipboard::new().and_then(|mut clipboard| {
            clipboard
                .get()
                .clipboard(arboard::LinuxClipboardKind::Primary)
                .text()
        }),
        deadline,
    )
}

#[cfg(not(target_os = "linux"))]
pub(super) fn read(_deadline: Instant) -> Result<String, String> {
    Err("PRIMARY selection unavailable".into())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn wayland_primary_does_not_require_xwayland() {
        assert!(display_available(true, false, || true));
        assert!(display_available(true, true, || true));
    }

    #[test]
    fn unsupported_wayland_does_not_fall_back_to_x11_primary() {
        // Arboard selects its Wayland backend before the requested clipboard kind.
        assert!(!display_available(true, true, || false));
        assert!(!display_available(true, false, || false));
    }

    #[test]
    fn x11_does_not_probe_wayland() {
        assert!(display_available(false, true, || panic!(
            "unexpected Wayland probe"
        )));
        assert!(!display_available(false, false, || panic!(
            "unexpected Wayland probe"
        )));
    }

    #[test]
    #[ignore = "requires a Wayland compositor without data-control PRIMARY support"]
    fn wayland_without_primary_stays_disabled() {
        assert!(std::env::var_os("WAYLAND_DISPLAY").is_some());
        assert!(!available());
    }

    #[test]
    #[ignore = "requires a disposable Wayland compositor and CODEX_TEST_ISOLATED_PRIMARY=1"]
    fn wayland_primary_round_trip_preserves_clipboard() {
        assert_eq!(
            std::env::var("CODEX_TEST_ISOLATED_PRIMARY").as_deref(),
            Ok("1")
        );
        assert!(std::env::var_os("WAYLAND_DISPLAY").is_some());
        assert!(available(), "compositor must support PRIMARY");
        let mut clipboard = arboard::Clipboard::new().expect("Wayland clipboard");
        clipboard
            .set_text("CLIPBOARD sentinel")
            .expect("set CLIPBOARD");
        let _primary_lease =
            copy("PRIMARY: русский текст\nsecond line", || Ok(())).expect("publish PRIMARY");
        assert_eq!(
            read(Instant::now() + std::time::Duration::from_secs(5)).expect("read PRIMARY"),
            "PRIMARY: русский текст\nsecond line",
        );
        assert_eq!(
            clipboard.get_text().expect("read CLIPBOARD"),
            "CLIPBOARD sentinel"
        );
    }
}

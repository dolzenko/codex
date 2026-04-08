use std::path::PathBuf;

use crate::clipboard_paste::PasteImageError;
use crate::clipboard_paste::PastedImageInfo;
use crate::clipboard_paste::paste_image_to_temp_png;
use crate::clipboard_text::ReadClipboardTextError;
use crate::clipboard_text::read_text_from_clipboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShortcutPasteRequest {
    CtrlV,
    AltV,
}

#[derive(Debug, Clone)]
pub(crate) enum ShortcutPasteAction {
    Text(String),
    Image {
        path: PathBuf,
        info: PastedImageInfo,
    },
    Error(String),
}

pub(crate) fn resolve_shortcut_paste(request: ShortcutPasteRequest) -> ShortcutPasteAction {
    let _ = request;

    match read_text_from_clipboard() {
        Ok(Some(text)) => ShortcutPasteAction::Text(text),
        Ok(None) => match paste_image_to_temp_png() {
            Ok((path, info)) => ShortcutPasteAction::Image { path, info },
            Err(PasteImageError::NoImage(_)) => ShortcutPasteAction::Error(
                "Failed to paste clipboard contents: no text or image available on the clipboard."
                    .to_string(),
            ),
            Err(err) => {
                ShortcutPasteAction::Error(format!("Failed to paste clipboard image: {err}"))
            }
        },
        Err(ReadClipboardTextError::ClipboardUnavailable(text_err)) => {
            match paste_image_to_temp_png() {
                Ok((path, info)) => ShortcutPasteAction::Image { path, info },
                Err(PasteImageError::NoImage(_)) => ShortcutPasteAction::Error(format!(
                    "Failed to paste clipboard text or image: clipboard text unavailable: {text_err}; no image on clipboard."
                )),
                Err(err) => ShortcutPasteAction::Error(format!(
                    "Failed to paste clipboard text or image: clipboard text unavailable: {text_err}; image paste failed: {err}"
                )),
            }
        }
    }
}

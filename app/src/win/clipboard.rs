//! OBS 用 URL を Windows のクリップボードへ渡す。
//!
//! GUI サブシステムのアプリなので、`clip.exe` を起動するとコンソール窓が一瞬光る。
//! それを避けるため Win32 を直接使う。

use anyhow::{bail, Context, Result};
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

/// Win32 の標準 CF_UNICODETEXT clipboard format。
const CF_UNICODE_TEXT: u32 = 13;

trait ClipboardBackend {
    fn set_unicode_text(&mut self, owner: HWND, text: &[u16]) -> Result<()>;
}

struct WindowsClipboard;

struct OpenClipboardGuard;

impl Drop for OpenClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

impl ClipboardBackend for WindowsClipboard {
    fn set_unicode_text(&mut self, owner: HWND, text: &[u16]) -> Result<()> {
        unsafe {
            OpenClipboard(Some(owner)).context("OpenClipboard")?;
            let _clipboard = OpenClipboardGuard;
            EmptyClipboard().context("EmptyClipboard")?;

            let byte_len = text
                .len()
                .checked_mul(std::mem::size_of::<u16>())
                .context("clipboard text is too large")?;
            let memory = GlobalAlloc(GMEM_MOVEABLE, byte_len).context("GlobalAlloc")?;
            let destination = GlobalLock(memory).cast::<u16>();
            if destination.is_null() {
                let _ = GlobalFree(Some(memory));
                bail!("GlobalLock failed");
            }
            std::ptr::copy_nonoverlapping(text.as_ptr(), destination, text.len());
            // 戻り値が 0 でも lock count が 0 になった正常系を含むため、ここでは無視する。
            let _ = GlobalUnlock(memory);

            if let Err(error) = SetClipboardData(CF_UNICODE_TEXT, Some(HANDLE(memory.0))) {
                let _ = GlobalFree(Some(HGLOBAL(memory.0)));
                return Err(error).context("SetClipboardData");
            }
        }
        Ok(())
    }
}

pub fn set_text(text: &str) -> Result<()> {
    copy_text_with(&mut WindowsClipboard, HWND::default(), text)
}

fn copy_text_with(backend: &mut impl ClipboardBackend, owner: HWND, text: &str) -> Result<()> {
    if text.contains('\0') {
        bail!("clipboard text contains a NUL character");
    }
    let encoded: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    backend
        .set_unicode_text(owner, &encoded)
        .context("クリップボードへコピーできません")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingClipboard {
        text: Vec<u16>,
    }

    impl ClipboardBackend for RecordingClipboard {
        fn set_unicode_text(&mut self, _owner: HWND, text: &[u16]) -> Result<()> {
            self.text = text.to_vec();
            Ok(())
        }
    }

    #[test]
    fn nul_終端の_utf16_へ変換する() {
        let mut clipboard = RecordingClipboard::default();
        copy_text_with(
            &mut clipboard,
            HWND::default(),
            "http://127.0.0.1:16874/board",
        )
        .unwrap();

        assert_eq!(clipboard.text.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&clipboard.text[..clipboard.text.len() - 1]).unwrap(),
            "http://127.0.0.1:16874/board"
        );
    }

    #[test]
    fn 埋め込まれた_nul_はクリップボードへ触る前に弾く() {
        let mut clipboard = RecordingClipboard::default();
        let error = copy_text_with(&mut clipboard, HWND::default(), "safe\0hidden").unwrap_err();
        assert!(error.to_string().contains("NUL"));
        assert!(clipboard.text.is_empty());
    }
}

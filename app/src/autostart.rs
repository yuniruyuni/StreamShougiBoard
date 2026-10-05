//! Windows 起動時の自動開始。HKCU の Run へ自分の exe を登録する。
//!
//! 状態はレジストリそのものを正とし、設定ファイルへは持たない。両方に持つと、
//! 片方を手で消したときに「設定上は on なのに起動しない」という食い違いが出る。
//!
//! 管理者権限は要らない (HKCU のみ)。失敗しても次にメニューを開けばチェックが
//! 外れたままなので、黙って成功したように見えることはない。

use std::path::Path;

/// Run へ書くコマンド行。空白を含むパスでも 1 つの実行ファイルとして扱われるよう必ず引用する。
pub(crate) fn command_line(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// 登録済みの値が、いまの exe を指しているか。
///
/// exe を別の場所へ移した後は「未登録」として見せる。そのまま on に見せると、
/// 消えた場所を指したまま起動しなくなる。1 回押せば今の場所へ書き直る。
pub(crate) fn points_at(stored: &str, exe: &Path) -> bool {
    fn normalize(value: &str) -> String {
        // Windows のパスは大文字小文字を区別せず、区切りも両方通る。
        value
            .trim()
            .trim_matches('"')
            .replace('/', "\\")
            .to_lowercase()
    }
    normalize(stored) == normalize(&exe.display().to_string())
}

#[cfg(windows)]
mod registry {
    use super::{command_line, points_at};
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    };

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// Run に並ぶ値の名前。Windows の「スタートアップ アプリ」にもこの名前で出る。
    const VALUE_NAME: &str = "StreamShougiBoard";

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                let _ = RegCloseKey(self.0);
            }
        }
    }

    fn open(access: REG_SAM_FLAGS) -> Option<Key> {
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(RUN_KEY),
                None,
                access,
                &mut key,
            )
        };
        (status == ERROR_SUCCESS).then_some(Key(key))
    }

    fn read(key: &Key) -> Option<String> {
        let name = HSTRING::from(VALUE_NAME);
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        let status =
            unsafe { RegQueryValueExW(key.0, &name, None, Some(&mut kind), None, Some(&mut size)) };
        if status != ERROR_SUCCESS || kind != REG_SZ || size == 0 {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                &name,
                None,
                None,
                Some(buffer.as_mut_ptr()),
                Some(&mut size),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        buffer.truncate(size as usize);
        let wide: Vec<u16> = buffer
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_ne_bytes(*pair))
            .collect();
        Some(
            String::from_utf16_lossy(&wide)
                .trim_end_matches('\0')
                .to_owned(),
        )
    }

    /// いまの exe が Run へ登録されているか。読めない環境では false として扱う。
    pub(crate) fn enabled() -> bool {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        open(KEY_QUERY_VALUE)
            .and_then(|key| read(&key))
            .is_some_and(|stored| points_at(&stored, &exe))
    }

    pub(crate) fn set(on: bool) -> anyhow::Result<()> {
        let key = open(KEY_SET_VALUE)
            .ok_or_else(|| anyhow::anyhow!("スタートアップ設定を開けませんでした"))?;
        let name = HSTRING::from(VALUE_NAME);
        let status = if on {
            let exe = std::env::current_exe()?;
            let value: Vec<u16> = command_line(&exe)
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // REG_SZ はバイト列で渡す。終端 NUL を含めた長さを数える。
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    value.as_ptr().cast::<u8>(),
                    std::mem::size_of_val(&value[..]),
                )
            };
            unsafe { RegSetValueExW(key.0, &name, None, REG_SZ, Some(bytes)) }
        } else {
            let status = unsafe { RegDeleteValueW(key.0, &name) };
            // 元から無いのは、消したい状態がすでに実現しているということ。
            if status == ERROR_FILE_NOT_FOUND {
                ERROR_SUCCESS
            } else {
                status
            }
        };
        anyhow::ensure!(
            status == ERROR_SUCCESS,
            "スタートアップ設定を書き換えられませんでした ({})",
            status.0
        );
        Ok(())
    }
}

#[cfg(windows)]
pub(crate) use registry::{enabled, set};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn 空白を含むパスでも一つの実行ファイルとして渡す() {
        let exe = PathBuf::from(r"C:\Program Files\Shougi Board\stream-shougi-board.exe");
        assert_eq!(
            command_line(&exe),
            "\"C:\\Program Files\\Shougi Board\\stream-shougi-board.exe\""
        );
    }

    #[test]
    fn 登録済みの値が今のexeを指しているかを見る() {
        let exe = PathBuf::from(r"D:\tools\StreamShougiBoard\stream-shougi-board.exe");
        assert!(points_at(&command_line(&exe), &exe));
        // Windows のパスは大文字小文字を区別せず、区切りも両方通る。
        assert!(points_at(
            r"d:\TOOLS\streamshougiboard\STREAM-SHOUGI-BOARD.EXE",
            &exe
        ));
        assert!(points_at(
            r"  D:/tools/StreamShougiBoard/stream-shougi-board.exe  ",
            &exe
        ));
        // 別の場所を指したままなら未登録として見せ、押し直しで今の場所へ直す。
        assert!(!points_at(r"C:\old\stream-shougi-board.exe", &exe));
        assert!(!points_at("", &exe));
    }
}

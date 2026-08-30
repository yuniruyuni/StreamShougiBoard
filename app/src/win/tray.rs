//! タスクトレイ常駐。
//!
//! 盤の操作はブラウザで開く操作画面が持つので、ここは常駐と導線だけを引き受ける。
//! メニュー: 操作画面を開く / OBS 用 URL をコピー / 第三者ライセンス / 終了。

use std::cell::RefCell;
use std::sync::Arc;

use anyhow::{Context, Result};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, KillTimer, LoadIconW, PostQuitMessage,
    RegisterClassW, SetForegroundWindow, SetTimer, TrackPopupMenu, TranslateMessage, HWND_MESSAGE,
    IDI_APPLICATION, MF_SEPARATOR, MF_STRING, MSG, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_RBUTTONUP,
    WM_TIMER, WNDCLASSW,
};

use crate::net::Hub;
use crate::platform::{copy_to_clipboard, open_in_browser};
use crate::Urls;

/// トレイからのコールバックメッセージ。
const WM_TRAY: u32 = WM_APP + 1;
const TRAY_ID: u32 = 1;
const TOOLTIP_TIMER_ID: usize = 1;
/// tooltip の接続数を見直す間隔。プロセス内の数を読むだけなので軽い。
const TOOLTIP_INTERVAL_MS: u32 = 1_000;

const MENU_OPEN_CONTROL: usize = 1;
const MENU_COPY_BOARD_URL: usize = 2;
const MENU_LICENSES: usize = 3;
const MENU_EXIT: usize = 4;

/// window proc から触るための、このスレッド限定の文脈。
struct TrayContext {
    hub: Arc<Hub>,
    urls: Urls,
    tooltip: String,
}

thread_local! {
    static CONTEXT: RefCell<Option<TrayContext>> = const { RefCell::new(None) };
}

/// tooltip に出す文言。接続数が変わったときだけ書き換える。
fn tooltip_for(url: &str, subscribers: usize) -> String {
    format!("StreamShougiBoard — {url}\nOBS 接続: {subscribers}")
}

fn notify_icon_data(hwnd: HWND, flags: u32) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        uFlags: windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS(flags),
        uCallbackMessage: WM_TRAY,
        ..Default::default()
    }
}

fn write_tip(data: &mut NOTIFYICONDATAW, tooltip: &str) {
    let encoded: Vec<u16> = tooltip.encode_utf16().chain(std::iter::once(0)).collect();
    let len = encoded.len().min(data.szTip.len());
    data.szTip[..len].copy_from_slice(&encoded[..len]);
    // 上限で切れた場合も必ず終端する。
    data.szTip[data.szTip.len() - 1] = 0;
}

/// exe へ埋め込んだアイコンを使う。取れない環境では既定のアプリアイコンへ落とす。
fn app_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
    unsafe {
        let instance = GetModuleHandleW(None).ok();
        if let Some(instance) = instance {
            // MAKEINTRESOURCE と同じで、リソース ID を整数のままポインタ位置へ載せる。
            // winresource が set_icon で付ける ID は 1。
            let resource_id = PCWSTR(std::ptr::without_provenance(1));
            if let Ok(icon) = LoadIconW(Some(instance.into()), resource_id) {
                return icon;
            }
        }
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

fn add_icon(hwnd: HWND, tooltip: &str) -> Result<()> {
    let mut data = notify_icon_data(hwnd, NIF_MESSAGE.0 | NIF_ICON.0 | NIF_TIP.0);
    data.hIcon = app_icon();
    write_tip(&mut data, tooltip);

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        anyhow::bail!("Shell_NotifyIconW(NIM_ADD) に失敗しました");
    }
    Ok(())
}

fn update_tip(hwnd: HWND, tooltip: &str) {
    let mut data = notify_icon_data(hwnd, NIF_TIP.0);
    write_tip(&mut data, tooltip);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn remove_icon(hwnd: HWND) {
    let data = notify_icon_data(hwnd, 0);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

/// 右クリックでメニューを出し、選ばれた項目を実行する。
fn show_menu(hwnd: HWND) {
    let selected = unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let _ = AppendMenuW(menu, MF_STRING, MENU_OPEN_CONTROL, w!("操作画面を開く"));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_COPY_BOARD_URL,
            w!("OBS 用 URL をコピー"),
        );
        let _ = AppendMenuW(menu, MF_STRING, MENU_LICENSES, w!("第三者ライセンス..."));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT, w!("終了"));

        let mut pos = POINT::default();
        let _ = GetCursorPos(&mut pos);
        // メニューを閉じられるようにするための定石 (フォーカスを一時的に取る)。
        let _ = SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
            pos.x,
            pos.y,
            None,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        selected.0 as usize
    };

    CONTEXT.with_borrow(|context| {
        let Some(context) = context else { return };
        match selected {
            MENU_OPEN_CONTROL => open_in_browser(&context.urls.control),
            MENU_COPY_BOARD_URL => copy_to_clipboard(&context.urls.board),
            MENU_LICENSES => open_in_browser(&context.urls.licenses),
            MENU_EXIT => unsafe { PostQuitMessage(0) },
            _ => {}
        }
    });
}

/// 接続数を読んで、変わっていたときだけ tooltip を書き換える。
fn refresh_tooltip(hwnd: HWND) {
    CONTEXT.with_borrow_mut(|context| {
        let Some(context) = context else { return };
        let next = tooltip_for(&context.urls.control, context.hub.subscriber_count());
        if next == context.tooltip {
            return;
        }
        context.tooltip = next;
        update_tip(hwnd, &context.tooltip);
    });
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let event = (lparam.0 as u32) & 0xffff;
            if event == WM_RBUTTONUP || event == WM_LBUTTONUP || event == WM_CONTEXTMENU {
                show_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == TOOLTIP_TIMER_ID => {
            refresh_tooltip(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// メッセージ専用ウィンドウを作ってトレイを常駐させ、「終了」が選ばれるまで戻らない。
pub fn run(hub: &Arc<Hub>, urls: &Urls) -> Result<()> {
    let tooltip = tooltip_for(&urls.control, hub.subscriber_count());
    CONTEXT.set(Some(TrayContext {
        hub: hub.clone(),
        urls: Urls {
            control: urls.control.clone(),
            board: urls.board.clone(),
            licenses: urls.licenses.clone(),
        },
        tooltip: tooltip.clone(),
    }));

    unsafe {
        let instance = GetModuleHandleW(None).context("GetModuleHandleW")?;
        let class_name = w!("StreamShougiBoardTray");

        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        // 同名クラスが既にある場合も続行できるよう、戻り値は失敗として扱わない。
        let _ = RegisterClassW(&class);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            &HSTRING::from("StreamShougiBoard"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            // 画面に出さないメッセージ専用ウィンドウ。
            Some(HWND_MESSAGE),
            None,
            Some(instance.into()),
            None,
        )
        .context("CreateWindowExW")?;

        add_icon(hwnd, &tooltip)?;
        SetTimer(Some(hwnd), TOOLTIP_TIMER_ID, TOOLTIP_INTERVAL_MS, None);

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let _ = KillTimer(Some(hwnd), TOOLTIP_TIMER_ID);
        remove_icon(hwnd);
        let _ = DestroyWindow(hwnd);
    }

    CONTEXT.set(None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_に_url_と接続数を出す() {
        let tooltip = tooltip_for("http://127.0.0.1:16874", 2);
        assert!(tooltip.contains("http://127.0.0.1:16874"));
        assert!(tooltip.contains("OBS 接続: 2"));
    }

    #[test]
    fn 長い_tooltip_も終端される() {
        let mut data = NOTIFYICONDATAW::default();
        write_tip(&mut data, &"あ".repeat(1000));
        assert_eq!(data.szTip[data.szTip.len() - 1], 0);
    }
}

//! Tudo que o webview não alcança no Windows: geometria da tela, estilo da janela, mouse global e COM.

use std::cell::Cell;

use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, WebviewWindow};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
};

use crate::layout::{Notch, Rect, WINDOW_SIZE};

/// Largura da Island em repouso, já que o Windows não tem Notch (o mesmo valor do Notch simulado do macOS).
const FAKE_NOTCH_WIDTH: f64 = 190.0;
/// Altura da Island em repouso: uma barra fina, do tamanho de uma barra de título compacta.
const FAKE_NOTCH_HEIGHT: f64 = 32.0;

/// Liga o COM (multithreaded) na thread atual, uma vez. WinRT e Core Audio exigem.
pub fn com_init() {
    thread_local! {
        static DONE: Cell<bool> = const { Cell::new(false) };
    }
    DONE.with(|done| {
        if !done.get() {
            // S_FALSE (já iniciado) e RPC_E_CHANGED_MODE (a thread já é STA) servem do mesmo jeito
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            done.set(true);
        }
    });
}

/// Nome mostrado nos Ajustes: `\\.\DISPLAY2` vira "Tela 2".
fn label(monitor: &Monitor) -> String {
    let name = monitor.name().cloned().unwrap_or_default();
    match name.strip_prefix(r"\\.\DISPLAY") {
        Some(n) if !n.is_empty() => format!("Tela {n}"),
        _ if name.is_empty() => "Tela".to_owned(),
        _ => name,
    }
}

pub fn screen_names(app: &AppHandle) -> Vec<String> {
    app.available_monitors().unwrap_or_default().iter().map(label).collect()
}

/// Target Screen: "auto" e "main" são a principal; um nome desconectado cai pra principal também.
/// Devolve o retângulo em pixels físicos, a escala (DPI) e o Notch simulado.
pub fn target_screen(app: &AppHandle, display: &str) -> Option<(Rect, f64, Notch)> {
    let monitors = app.available_monitors().ok()?;
    let named = match display {
        "auto" | "main" => None,
        name => monitors.iter().find(|m| label(m) == name),
    };
    let monitor = match named {
        Some(m) => m.clone(),
        None => app.primary_monitor().ok().flatten().or_else(|| monitors.first().cloned())?,
    };
    let (pos, size) = (monitor.position(), monitor.size());
    let rect = Rect { x: pos.x as f64, y: pos.y as f64, w: size.width as f64, h: size.height as f64 };
    let notch = Notch { width: FAKE_NOTCH_WIDTH, height: FAKE_NOTCH_HEIGHT, has_notch: false };
    Some((rect, monitor.scale_factor(), notch))
}

/// Janela fora da barra de tarefas e do Alt+Tab, sempre acima das outras.
pub fn configure_window(win: &WebviewWindow) {
    let Ok(hwnd) = win.hwnd() else { return };
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = (style & !(WS_EX_APPWINDOW.0 as isize)) | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Cola a janela no topo central da tela. `ui` é o "Tamanho da Island": a janela e o zoom do webview crescem juntos,
/// então o frontend continua desenhando em 500x340 px.
pub fn place_window(win: &WebviewWindow, screen: Rect, scale: f64, ui: f64) {
    let (w, h) = (WINDOW_SIZE.0 * ui, WINDOW_SIZE.1 * ui);
    let _ = win.set_zoom(ui);
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let x = screen.x + (screen.w - w * scale) / 2.0;
    let _ = win.set_position(PhysicalPosition::new(x.round() as i32, screen.y.round() as i32));
}

/// Posição global do cursor em pixels físicos, no mesmo espaço de `Rect`.
pub fn mouse_location(app: &AppHandle) -> (f64, f64) {
    app.cursor_position().map(|p| (p.x, p.y)).unwrap_or((f64::MIN, f64::MIN))
}

/// Garante que a janela principal continua topmost. Só mexe se ela perdeu o estado: reafirmar sempre a jogaria
/// por cima de tudo o que está no mesmo nível, como o menu de contexto aberto sobre ela.
pub fn refresh_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window(crate::MAIN_LABEL) else { return };
    let Ok(hwnd) = win.hwnd() else { return };
    let topmost = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } & WS_EX_TOPMOST.0 as isize != 0;
    if !topmost {
        configure_window(&win);
    }
}

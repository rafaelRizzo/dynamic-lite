//! Tudo que o webview não alcança: geometria do Notch, nível da janela, mouse global, notificações.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplicationDidChangeScreenParametersNotification, NSEvent, NSMainMenuWindowLevel,
    NSRunningApplication, NSScreen, NSWindow, NSWindowCollectionBehavior,
};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSNotificationName,
    NSOperationQueue, NSPoint, NSRect, NSSize, NSString,
};

use crate::layout::{Notch, Rect};

/// Largura de um Notch simulado em telas sem recorte (próximo ao de um MacBook Pro 14").
const FAKE_NOTCH_WIDTH: f64 = 190.0;
const FALLBACK_BAR_HEIGHT: f64 = 24.0;

/// Tela principal (a da menu bar) e o Notch dela, real ou simulado.
pub fn primary_screen(mtm: MainThreadMarker) -> Option<(Rect, Notch)> {
    let screen = NSScreen::screens(mtm).firstObject()?;
    let f = screen.frame();
    let rect = Rect { x: f.origin.x, y: f.origin.y, w: f.size.width, h: f.size.height };
    let inset = screen.safeAreaInsets().top;

    let notch = if inset > 0.0 {
        let left = screen.auxiliaryTopLeftArea().size.width;
        let right = screen.auxiliaryTopRightArea().size.width;
        Notch { width: f.size.width - left - right, height: inset, has_notch: true }
    } else {
        let vf = screen.visibleFrame();
        let bar = (f.origin.y + f.size.height) - (vf.origin.y + vf.size.height);
        Notch {
            width: FAKE_NOTCH_WIDTH,
            height: if bar > 0.0 { bar } else { FALLBACK_BAR_HEIGHT },
            has_notch: false,
        }
    };
    Some((rect, notch))
}

/// Janela acima da menu bar, presente em todos os Spaces e sobre apps em tela cheia.
pub fn configure_window(ns_window: *mut std::ffi::c_void) {
    let win = unsafe { &*(ns_window as *const NSWindow) };
    win.setLevel(NSMainMenuWindowLevel + 3);
    win.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::IgnoresCycle,
    );
    win.setHasShadow(false);
}

/// Cola a janela no topo central da tela (coordenadas Cocoa, origem embaixo à esquerda).
pub fn place_window(ns_window: *mut std::ffi::c_void, screen: Rect, size: (f64, f64)) {
    let win = unsafe { &*(ns_window as *const NSWindow) };
    let (w, h) = size;
    let frame = NSRect::new(
        NSPoint::new(screen.x + (screen.w - w) / 2.0, screen.y + screen.h - h),
        NSSize::new(w, h),
    );
    win.setFrame_display(frame, true);
}

/// Posição global do cursor em pontos, no mesmo espaço de `NSScreen.frame`.
pub fn mouse_location() -> (f64, f64) {
    let p = NSEvent::mouseLocation();
    (p.x, p.y)
}

pub fn is_app_running(bundle_id: &str) -> bool {
    let id = NSString::from_str(bundle_id);
    NSRunningApplication::runningApplicationsWithBundleIdentifier(&id)
        .iter()
        .any(|app| !app.isTerminated())
}

fn observe(
    center: &NSNotificationCenter,
    name: &NSNotificationName,
    handler: impl Fn() + 'static,
) {
    let block = RcBlock::new(move |_: NonNull<NSNotification>| handler());
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(name),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    // observador vive o app inteiro
    std::mem::forget(token);
}

/// Monitor conectado/removido, resolução ou menu bar mudou.
pub fn on_screen_change(handler: impl Fn() + 'static) {
    let center = NSNotificationCenter::defaultCenter();
    observe(&center, unsafe { NSApplicationDidChangeScreenParametersNotification }, handler);
}

/// O Spotify publica isso a cada play/pause/troca de faixa.
pub fn on_spotify_change(handler: impl Fn() + 'static) {
    let center = NSDistributedNotificationCenter::defaultCenter();
    let name = NSString::from_str("com.spotify.client.PlaybackStateChanged");
    observe(&center, &name, handler);
}

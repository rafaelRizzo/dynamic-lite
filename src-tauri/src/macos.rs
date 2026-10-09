//! Tudo que o webview não alcança: geometria do Notch, nível da janela, mouse global, notificações.

use std::cell::{Cell, RefCell};
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAccessibility, NSAccessibilityFloatingWindowSubrole, NSAnimatablePropertyContainer,
    NSAnimationContext, NSApplication,
    NSApplicationDidChangeScreenParametersNotification, NSColor, NSEvent, NSGlassEffectView,
    NSGlassEffectViewStyle, NSHapticFeedbackManager, NSHapticFeedbackPattern,
    NSHapticFeedbackPerformanceTime, NSHapticFeedbackPerformer, NSMainMenuWindowLevel,
    NSRunningApplication, NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{
    NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSNotificationName,
    NSOperationQueue, NSPoint, NSRect, NSSize, NSString,
};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGMutablePath, CGPath};
use objc2_quartz_core::{
    CAMediaTiming, CAMediaTimingFunction, CAShapeLayer, CASpringAnimation, CATransaction,
};
use serde::Deserialize;

use crate::layout::{Notch, Rect};

/// Largura de um Notch simulado em telas sem recorte (próximo ao de um MacBook Pro 14").
const FAKE_NOTCH_WIDTH: f64 = 190.0;
const FALLBACK_BAR_HEIGHT: f64 = 24.0;

fn has_notch(screen: &NSScreen) -> bool {
    screen.safeAreaInsets().top > 0.0
}

/// Target Screen: "auto" (com Notch, senão a principal), "main" ou o nome da tela.
/// Nome desconectado cai para "auto".
pub fn target_screen(mtm: MainThreadMarker, display: &str) -> Option<(Rect, Notch)> {
    let screens = NSScreen::screens(mtm);
    let named = match display {
        "auto" => None,
        "main" => screens.firstObject(),
        name => screens.iter().find(|s| s.localizedName().to_string() == name),
    };
    let screen = named
        .or_else(|| screens.iter().find(|s| has_notch(s)))
        .or_else(|| screens.firstObject())?;
    Some(geometry(&screen))
}

pub fn screen_names(mtm: MainThreadMarker) -> Vec<String> {
    NSScreen::screens(mtm).iter().map(|s| s.localizedName().to_string()).collect()
}

/// Frame da tela e o Notch dela, real ou simulado.
fn geometry(screen: &NSScreen) -> (Rect, Notch) {
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
    (rect, notch)
}

/// Janela acima da menu bar, presente em todos os Spaces e sobre apps em tela cheia.
pub fn configure_window(ns_window: *mut std::ffi::c_void) {
    let win = unsafe { &*(ns_window as *const NSWindow) };
    win.setLevel(NSMainMenuWindowLevel + 3);
    win.setHasShadow(false);
    // seletores de janela (AltTab e afins) descartam AXFloatingWindow antes de qualquer outra regra;
    // sem isso, ao receber clique ela vira AXMain e o AltTab a admite mesmo acima da menu bar
    win.setAccessibilitySubrole(Some(unsafe { NSAccessibilityFloatingWindowSubrole }));
}

/// Comportamento nas mesas (Spaces) e no Mission Control.
/// `all_spaces`: aparece em todas as mesas; senão fica só na mesa onde o app abriu.
/// `hide_in_mission_control`: `Transient` some no Mission Control (a barra de mesas fica no topo, onde a
/// Island está); `Stationary` continua visível e parada nele.
pub fn set_spaces_behavior(ns_window: *mut std::ffi::c_void, all_spaces: bool, hide_in_mission_control: bool) {
    let win = unsafe { &*(ns_window as *const NSWindow) };
    let mut behavior = NSWindowCollectionBehavior::FullScreenAuxiliary | NSWindowCollectionBehavior::IgnoresCycle;
    if all_spaces {
        behavior |= NSWindowCollectionBehavior::CanJoinAllSpaces;
    }
    behavior |= if hide_in_mission_control {
        NSWindowCollectionBehavior::Transient
    } else {
        NSWindowCollectionBehavior::Stationary
    };
    win.setCollectionBehavior(behavior);
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

#[allow(deprecated)] // `activate()` só existe no macOS 14+
pub fn activate_app() {
    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    }
}

pub fn haptic() {
    NSHapticFeedbackManager::defaultPerformer().performFeedbackPattern_performanceTime(
        NSHapticFeedbackPattern::Alignment,
        NSHapticFeedbackPerformanceTime::Now,
    );
}

// ---------- Liquid Glass (ADR 0004) ----------

/// Forma da Island centrada no topo da janela: a mesma geometria do Style Black, Ears incluídas.
#[derive(Clone, Copy, Deserialize)]
pub struct GlassShape {
    pub width: f64,
    pub height: f64,
    pub radius: f64,
    pub ear: f64,
}

struct Glass {
    view: Retained<NSGlassEffectView>,
    mask: Retained<CAShapeLayer>,
}

thread_local! {
    static GLASS: RefCell<Option<Glass>> = const { RefCell::new(None) };
    /// Invalida o "esconder ao terminar" se um novo show começar no meio da animação.
    static GLASS_GEN: Cell<u64> = const { Cell::new(0) };
}

/// Quanto o vidro passa do topo da janela: a borda especular dele fica fora da tela.
const GLASS_OVERSCAN: f64 = 40.0;

/// Mesma mola do React (Motion): o vidro e o conteúdo andam juntos.
const SPRING: (f64, f64, f64) = (380.0, 26.0, 0.9);

pub fn glass_supported() -> bool {
    AnyClass::get(c"NSGlassEffectView").is_some()
}

fn animate(duration: f64, curve: [f32; 4], changes: impl Fn() + 'static, done: Option<Box<dyn Fn()>>) {
    let changes = RcBlock::new(move |ctx: NonNull<NSAnimationContext>| {
        let ctx = unsafe { ctx.as_ref() };
        ctx.setDuration(duration);
        let timing = CAMediaTimingFunction::functionWithControlPoints(curve[0], curve[1], curve[2], curve[3]);
        ctx.setTimingFunction(Some(&timing));
        changes();
    });
    match done {
        Some(done) => {
            let done = RcBlock::new(move || done());
            NSAnimationContext::runAnimationGroup_completionHandler(&changes, Some(&done));
        }
        None => NSAnimationContext::runAnimationGroup(&changes),
    }
}

/// Contorno da Island (Ears côncavas + cantos inferiores) em coordenadas de layer (y pra cima).
/// Sempre a mesma sequência de elementos, pra o Core Animation conseguir interpolar entre formas.
fn island_path(s: GlassShape, view_w: f64, view_h: f64) -> CFRetained<CGMutablePath> {
    let cx = view_w / 2.0;
    let (l, r) = (cx - s.width / 2.0, cx + s.width / 2.0);
    let (top, bottom) = (0.0, s.height.max(0.0));
    let e = s.ear.max(0.0);
    let rad = s.radius.min(s.height / 2.0).min(s.width / 2.0).max(0.0);
    let y = |v: f64| view_h - v;

    let path = CGMutablePath::new();
    let p = Some(&*path);
    let m = std::ptr::null();
    unsafe {
        CGMutablePath::move_to_point(p, m, l - e, y(top));
        CGMutablePath::add_line_to_point(p, m, r + e, y(top));
        CGMutablePath::add_quad_curve_to_point(p, m, r, y(top), r, y(top + e));
        CGMutablePath::add_line_to_point(p, m, r, y(bottom - rad));
        CGMutablePath::add_quad_curve_to_point(p, m, r, y(bottom), r - rad, y(bottom));
        CGMutablePath::add_line_to_point(p, m, l + rad, y(bottom));
        CGMutablePath::add_quad_curve_to_point(p, m, l, y(bottom), l, y(bottom - rad));
        CGMutablePath::add_line_to_point(p, m, l, y(top + e));
        CGMutablePath::add_quad_curve_to_point(p, m, l, y(top), l - e, y(top));
    }
    CGMutablePath::close_subpath(p);
    path
}

fn as_object(path: &CGPath) -> &AnyObject {
    // CGPath é toll-free bridged com NSObject
    unsafe { &*(path as *const CGPath as *const AnyObject) }
}

/// Morph com mola a partir de onde a máscara está agora (mesmo no meio de outra animação).
fn morph(mask: &CAShapeLayer, to: &CGPath) {
    let from = unsafe { mask.presentationLayer() }
        .and_then(|layer| layer.downcast::<CAShapeLayer>().ok())
        .and_then(|layer| layer.path())
        .or_else(|| mask.path());

    CATransaction::begin();
    CATransaction::setDisableActions(true);
    mask.setPath(Some(to));
    if let Some(from) = from {
        let anim = CASpringAnimation::animationWithKeyPath(Some(&NSString::from_str("path")));
        let (stiffness, damping, mass) = SPRING;
        anim.setStiffness(stiffness);
        anim.setDamping(damping);
        anim.setMass(mass);
        unsafe {
            anim.setFromValue(Some(as_object(&from)));
            anim.setToValue(Some(as_object(to)));
        }
        anim.setDuration(anim.settlingDuration());
        mask.addAnimation_forKey(&anim, Some(&NSString::from_str("morph")));
    }
    CATransaction::commit();
}

/// Expanded de vidro com a forma `target`; `rest` é a forma de onde ele nasce e pra onde volta
/// (Compact ou Idle). `target: None` recolhe e esconde.
pub fn set_glass(mtm: MainThreadMarker, ns_window: *mut std::ffi::c_void, target: Option<GlassShape>, rest: GlassShape) {
    if !glass_supported() {
        return;
    }
    let win = unsafe { &*(ns_window as *const NSWindow) };
    let Some(content) = win.contentView() else { return };
    let bounds = content.bounds();
    let (w, h) = (bounds.size.width, bounds.size.height);

    let (view, mask) = GLASS.with(|cell| {
        let mut slot = cell.borrow_mut();
        let glass = slot.get_or_insert_with(|| {
            let view = NSGlassEffectView::new(mtm);
            view.setStyle(NSGlassEffectViewStyle::Regular);
            // escurece o vidro pra texto branco ler bem sobre fundos claros
            view.setTintColor(Some(&NSColor::colorWithWhite_alpha(0.0, 0.35)));
            view.setCornerRadius(0.0);
            view.setWantsLayer(true);
            view.setHidden(true);
            view.setAlphaValue(0.0);
            content.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, None);
            let mask = CAShapeLayer::new();
            if let Some(layer) = view.layer() {
                unsafe { layer.setMask(Some(&mask)) };
            }
            Glass { view, mask }
        });
        (glass.view.clone(), glass.mask.clone())
    });

    // o vidro cobre a janela toda e passa do topo; quem dá a forma é a máscara.
    // O contorno usa `h` como topo, então o trecho acima (com a borda brilhante) fica recortado.
    let origin_y = if content.isFlipped() { -GLASS_OVERSCAN } else { 0.0 };
    view.setFrame(NSRect::new(NSPoint::new(0.0, origin_y), NSSize::new(w, h + GLASS_OVERSCAN)));
    CATransaction::begin();
    CATransaction::setDisableActions(true);
    mask.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h + GLASS_OVERSCAN)));
    CATransaction::commit();

    let generation = GLASS_GEN.with(|g| {
        g.set(g.get() + 1);
        g.get()
    });
    let rest_path = island_path(rest, w, h);

    match target {
        Some(shape) => {
            if view.isHidden() {
                CATransaction::begin();
                CATransaction::setDisableActions(true);
                mask.setPath(Some(&rest_path));
                CATransaction::commit();
                view.setHidden(false);
            }
            morph(&mask, &island_path(shape, w, h));
            let v = view.clone();
            animate(0.14, [0.2, 0.0, 0.2, 1.0], move || v.animator().setAlphaValue(1.0), None);
        }
        None => {
            if view.isHidden() {
                return;
            }
            morph(&mask, &rest_path);
            let v = view.clone();
            let hide = view.clone();
            animate(0.3, [0.6, 0.0, 0.8, 1.0], move || v.animator().setAlphaValue(0.0), Some(Box::new(move || {
                if GLASS_GEN.with(|g| g.get()) == generation {
                    hide.setHidden(true);
                }
            })));
        }
    }
}

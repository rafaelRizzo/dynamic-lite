//! Com a Island aberta e o mouse nela, engole os gestos de trackpad do sistema (trocar de mesa, Mission Control...)
//! pra o deslize de 3 dedos só trocar de aba. Event tap que descarta os eventos "dock swipe" (tipo 30, subtipo 23),
//! os que o Dock usa pros gestos de 3 e 4 dedos; pinça e rotação passam.
//! Precisa da permissão de Acessibilidade: sem ela o tap não existe e o sistema continua reagindo ao gesto.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
    static kAXTrustedCheckOptionPrompt: *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDictionaryCreate(
        alloc: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        count: isize,
        key_callbacks: *const u8,
        value_callbacks: *const u8,
    ) -> *const c_void;
    fn CFRelease(object: *const c_void);
    fn CFMachPortCreateRunLoopSource(alloc: *const c_void, port: *mut c_void, order: isize) -> *mut c_void;
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    static kCFRunLoopCommonModes: *const c_void;
    static kCFBooleanTrue: *const c_void;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(*mut c_void, u32, *mut c_void, *mut c_void) -> *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
}

const SESSION_TAP: u32 = 1;
const HEAD_INSERT: u32 = 0;
/// tap ativo (pode descartar eventos), não só ouvinte
const DEFAULT_OPTIONS: u32 = 0;
/// o Dock recebe os gestos de sistema como eventos deste tipo, com este subtipo (conferido com um sensor no trackpad)
const DOCK_GESTURE: u32 = 30;
const SUBTYPE_FIELD: u32 = 110;
const DOCK_SWIPE_SUBTYPE: i64 = 23;
const DISABLED_BY_TIMEOUT: u32 = 0xFFFF_FFFE;
const DISABLED_BY_USER: u32 = 0xFFFF_FFFF;
const RETRY_EVERY: Duration = Duration::from_secs(3);

/// O frontend liga quando a Island está aberta com abas e o gesto de 3 dedos ativo.
static CAPTURE: AtomicBool = AtomicBool::new(false);
static TAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static MOUSE_ON_ISLAND: OnceLock<Box<dyn Fn() -> bool + Send + Sync>> = OnceLock::new();

pub fn set_capture(on: bool) {
    CAPTURE.store(on, Ordering::Relaxed);
}

extern "C" fn on_event(_proxy: *mut c_void, kind: u32, event: *mut c_void, _user: *mut c_void) -> *mut c_void {
    match kind {
        // o sistema desliga tap lento ou interrompido: religa
        DISABLED_BY_TIMEOUT | DISABLED_BY_USER => {
            let tap = TAP.load(Ordering::Relaxed);
            if !tap.is_null() {
                unsafe { CGEventTapEnable(tap, true) };
            }
            event
        }
        DOCK_GESTURE if CAPTURE.load(Ordering::Relaxed) => {
            // SAFETY: `event` é válido durante o callback
            let subtype = unsafe { CGEventGetIntegerValueField(event, SUBTYPE_FIELD) };
            if subtype == DOCK_SWIPE_SUBTYPE && MOUSE_ON_ISLAND.get().is_some_and(|f| f()) {
                std::ptr::null_mut()
            } else {
                event
            }
        }
        _ => event,
    }
}

/// Pede a permissão de Acessibilidade (abre o diálogo do sistema).
fn prompt_for_access() {
    // SAFETY: dicionário de um par chave/valor com as constantes do CoreFoundation, liberado em seguida
    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        );
        AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
    }
}

/// Instala o tap numa thread própria assim que a permissão existir. `wanted`: o gesto de 3 dedos está ligado.
/// `mouse_on_island`: o mouse está na Hit Region.
pub fn start(wanted: impl Fn() -> bool + Send + 'static, mouse_on_island: impl Fn() -> bool + Send + Sync + 'static) {
    if MOUSE_ON_ISLAND.set(Box::new(mouse_on_island)).is_err() {
        return;
    }
    thread::spawn(move || {
        let mut prompted = false;
        loop {
            if wanted() {
                // SAFETY: AXIsProcessTrusted não tem pré-condições
                if unsafe { AXIsProcessTrusted() } {
                    run_tap();
                } else if !prompted {
                    prompted = true;
                    prompt_for_access();
                }
            }
            thread::sleep(RETRY_EVERY);
        }
    });
}

/// Cria o tap e roda o run loop da thread; só volta se o tap não puder ser criado.
fn run_tap() {
    // SAFETY: o tap e a source vivem até o fim do processo (o run loop não retorna com a source instalada)
    unsafe {
        let mask = 1u64 << DOCK_GESTURE;
        let tap = CGEventTapCreate(SESSION_TAP, HEAD_INSERT, DEFAULT_OPTIONS, mask, on_event, std::ptr::null_mut());
        if tap.is_null() {
            return;
        }
        let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        if source.is_null() {
            return;
        }
        TAP.store(tap, Ordering::Relaxed);
        CFRunLoopAddSource(CFRunLoopGetCurrent(), source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);
        CFRunLoopRun();
        TAP.store(std::ptr::null_mut(), Ordering::Relaxed);
    }
}

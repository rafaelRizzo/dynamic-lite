//! Toque bruto do trackpad: detecta 3 dedos deslizando de lado, sem depender dos gestos do sistema.
//! Usa o framework privado MultitouchSupport, carregado em runtime (como o Process Tap do equalizer, ADR 0005):
//! se ele sumir numa versão do macOS, nada acontece e o resto do app segue.

use std::ffi::{c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
}

/// Um dedo no trackpad. Layout do `MTTouch` (96 bytes), conferido com os projetos que usam o framework.
#[repr(C)]
struct Touch {
    frame: i32,
    timestamp: f64,
    identifier: i32,
    /// 3 = encostando, 4 = apoiado; os outros valores são "levantando", "pairando"...
    state: i32,
    finger: i32,
    hand: i32,
    /// posição normalizada: 0-1, origem embaixo à esquerda do trackpad
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    size: f32,
    zero1: i32,
    angle: f32,
    major: f32,
    minor: f32,
    mm: [f32; 4],
    zero2: [i32; 2],
    unknown: f32,
}

type Callback = extern "C" fn(*mut c_void, *const Touch, c_int, f64, c_int) -> c_int;
type CreateList = unsafe extern "C" fn() -> *const c_void;
type Register = unsafe extern "C" fn(*mut c_void, Callback);
type Start = unsafe extern "C" fn(*mut c_void, c_int);

/// Quanto os 3 dedos precisam andar (fração da largura do trackpad, ~7 mm: dispara no começo do movimento, sem esperar o fim) pra contar como um deslize.
const DISTANCE: f32 = 0.05;

/// Gesto em andamento: onde os 3 dedos começaram.
#[derive(Default)]
struct Gesture {
    active: bool,
    fired: bool,
    start: (f32, f32),
}

static GESTURE: Mutex<Gesture> = Mutex::new(Gesture { active: false, fired: false, start: (0.0, 0.0) });
static HANDLER: OnceLock<Box<dyn Fn(bool) + Send + Sync>> = OnceLock::new();

/// `true` se os dedos foram pra esquerda (avançar), `false` pra direita.
fn swipe_direction(start: (f32, f32), now: (f32, f32)) -> Option<bool> {
    let (dx, dy) = (now.0 - start.0, now.1 - start.1);
    // tem que ser mais horizontal que vertical, senão é rolagem ou outro gesto
    (dx.abs() >= DISTANCE && dx.abs() > dy.abs() * 1.5).then_some(dx < 0.0)
}

extern "C" fn on_frame(_device: *mut c_void, touches: *const Touch, count: c_int, _timestamp: f64, _frame: c_int) -> c_int {
    let mut gesture = GESTURE.lock().unwrap();
    if touches.is_null() || !(1..=16).contains(&count) {
        gesture.active = false;
        return 0;
    }
    // SAFETY: o framework entrega `count` toques consecutivos, válidos durante o callback
    let all = unsafe { std::slice::from_raw_parts(touches, count as usize) };
    let down: Vec<&Touch> = all.iter().filter(|t| t.state == 3 || t.state == 4).collect();
    // só com exatamente 3 dedos: 2 é rolagem e 4 costuma ser gesto do sistema
    if down.len() != 3 {
        gesture.active = false;
        return 0;
    }
    let centroid = (down.iter().map(|t| t.x).sum::<f32>() / 3.0, down.iter().map(|t| t.y).sum::<f32>() / 3.0);
    if !gesture.active {
        *gesture = Gesture { active: true, fired: false, start: centroid };
    } else if !gesture.fired {
        if let Some(left) = swipe_direction(gesture.start, centroid) {
            gesture.fired = true;
            drop(gesture);
            if let Some(handler) = HANDLER.get() {
                handler(left);
            }
        }
    }
    0
}

/// Começa a ouvir todos os trackpads. `handler(true)`: 3 dedos foram pra esquerda; `false`: pra direita.
/// Devolve `false` se o framework não está disponível.
pub fn start(handler: impl Fn(bool) + Send + Sync + 'static) -> bool {
    if HANDLER.set(Box::new(handler)).is_err() {
        return false;
    }
    // SAFETY: símbolos resolvidos com dlsym e chamados com as assinaturas conhecidas do framework
    unsafe {
        let lib = dlopen(c"/System/Library/PrivateFrameworks/MultitouchSupport.framework/MultitouchSupport".as_ptr(), 1);
        if lib.is_null() {
            return false;
        }
        let (Some(create), Some(register), Some(start)) = (
            std::ptr::NonNull::new(dlsym(lib, c"MTDeviceCreateList".as_ptr())),
            std::ptr::NonNull::new(dlsym(lib, c"MTRegisterContactFrameCallback".as_ptr())),
            std::ptr::NonNull::new(dlsym(lib, c"MTDeviceStart".as_ptr())),
        ) else {
            return false;
        };
        let create: CreateList = std::mem::transmute(create.as_ptr());
        let register: Register = std::mem::transmute(register.as_ptr());
        let start: Start = std::mem::transmute(start.as_ptr());

        let devices = create();
        if devices.is_null() {
            return false;
        }
        for i in 0..CFArrayGetCount(devices) {
            let device = CFArrayGetValueAtIndex(devices, i) as *mut c_void;
            register(device, on_frame);
            start(device, 0);
        }
        CFArrayGetCount(devices) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confere que o framework carrega e acha trackpads: `cargo test framework_loads -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn framework_loads() {
        println!("start() = {}", start(|left| println!("deslize, esquerda = {left}")));
        // dá tempo de chegar alguns quadros, se houver dedos no trackpad
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    #[test]
    fn touch_layout_matches_the_framework() {
        assert_eq!(std::mem::size_of::<Touch>(), 96);
        assert_eq!(std::mem::offset_of!(Touch, state), 20);
        assert_eq!(std::mem::offset_of!(Touch, x), 32);
    }

    #[test]
    fn swipes_need_distance_and_a_horizontal_path() {
        assert_eq!(swipe_direction((0.5, 0.5), (0.44, 0.52)), Some(true));
        assert_eq!(swipe_direction((0.5, 0.5), (0.65, 0.5)), Some(false));
        assert_eq!(swipe_direction((0.5, 0.5), (0.53, 0.5)), None);
        // mais vertical que horizontal: não é deslize lateral
        assert_eq!(swipe_direction((0.5, 0.5), (0.6, 0.8)), None);
    }
}

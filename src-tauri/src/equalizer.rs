//! Equalizer ao vivo (ADR 0005, 0010): captura o áudio do Spotify, ou de todo o sistema quando o Spotify não
//! toca e a opção está ligada, com um Core Audio Process Tap (macOS 14.2+) e reduz a 4 bandas de energia,
//! emitidas pro frontend. Sem suporte ou sem permissão, nada é emitido e o frontend mantém a animação CSS.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::mem::{size_of, MaybeUninit};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::AllocAnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceIsStackedKey,
    kAudioAggregateDeviceMainSubDeviceKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDeviceSubDeviceListKey, kAudioAggregateDeviceTapAutoStartKey,
    kAudioAggregateDeviceTapListKey, kAudioAggregateDeviceUIDKey, kAudioDevicePropertyDeviceUID,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyProcessObjectList,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject,
    kAudioProcessPropertyBundleID, kAudioProcessPropertyIsRunningOutput, kAudioProcessPropertyPID, kAudioSubDeviceUIDKey, kAudioSubTapDriftCompensationKey,
    kAudioSubTapUIDKey, kAudioTapPropertyFormat, AudioDeviceCreateIOProcID,
    AudioDeviceDestroyIOProcID, AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop,
    AudioHardwareCreateAggregateDevice, AudioHardwareDestroyAggregateDevice, AudioObjectGetPropertyData,
    AudioObjectGetPropertyDataSize, AudioObjectID, AudioObjectPropertyAddress,
    AudioObjectPropertySelector, CATapDescription, CATapMuteBehavior,
};
use objc2_core_audio_types::{
    kAudioFormatFlagIsFloat, kAudioFormatLinearPCM, AudioBuffer, AudioBufferList,
    AudioStreamBasicDescription, AudioTimeStamp,
};
use objc2_core_foundation::{CFDictionary, CFRetained, CFString};
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::SharedSettings;

pub const LEVELS_EVENT: &str = "equalizer://levels";
/// `bool`: há áudio audível no sistema (só com a fonte Sistema).
pub const SYSTEM_EVENT: &str = "equalizer://system";
/// Inclui os helpers do Spotify (`com.spotify.client.helper...`).
const BUNDLE_PREFIX: &str = "com.spotify.client";
/// Quadros por segundo, do envio dos níveis e do desenho no frontend (lido por `equalizer_fps`).
const FPS: u64 = 24;
const FRAME: Duration = Duration::from_millis(1000 / FPS);
/// Variação mínima (0-1) em alguma barra pra valer um novo evento; abaixo disso o olho não vê.
const MIN_DELTA: f32 = 0.015;
/// Tocando sem tap (Spotify ainda sem processo de áudio, device ocupado): tenta de novo.
const RETRY: Duration = Duration::from_secs(5);
/// Com a fonte Sistema ligada, de quanto em quanto tempo se olha se algum app começou/parou de tocar.
const POLL: Duration = Duration::from_secs(1);
/// Sem sinal por esse tempo, o áudio do sistema conta como parado (pausas e silêncios curtos não piscam a Island).
const SYSTEM_HOLD: Duration = Duration::from_secs(2);

/// Centro (Hz) de cada barra, da esquerda pra direita: grave, médio-grave, médio, agudo.
const BANDS: [f32; 4] = [80.0, 350.0, 1500.0, 6000.0];
const BAND_Q: f32 = 1.0;
const ATTACK: f32 = 0.008;
const RELEASE: f32 = 0.06;
/// Média recente de cada banda: a barra mostra o quanto o som sobe/desce em relação a ela (batidas),
/// não o volume absoluto, que em música comprimida fica sempre no topo.
const AVERAGE: f32 = 0.8;
/// Desvio da média que leva a barra do meio ao topo (ou ao fundo).
const PUNCH_DB: f32 = 5.0;
/// Meia-vida do pico: trechos baixos (intro, fade) encolhem as barras.
const PEAK_DECAY: f32 = 3.0;
/// Abaixo do pico, quanto até a barra zerar.
const RANGE_DB: f32 = 30.0;
/// ~-80 dBFS: abaixo disso é silêncio (ou tap sem permissão, que entrega zeros).
const SILENCE: f32 = 1e-4;

// ---------- Símbolos do macOS 14.2+ ----------

type CreateTap = unsafe extern "C-unwind" fn(*const CATapDescription, *mut AudioObjectID) -> i32;
type DestroyTap = unsafe extern "C-unwind" fn(AudioObjectID) -> i32;

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// Resolvidos em runtime: linkar direto quebraria o app no macOS < 14.2.
fn tap_api() -> Option<(CreateTap, DestroyTap)> {
    static API: OnceLock<Option<(CreateTap, DestroyTap)>> = OnceLock::new();
    *API.get_or_init(|| unsafe {
        AnyClass::get(c"CATapDescription")?;
        let lib = dlopen(c"/System/Library/Frameworks/CoreAudio.framework/CoreAudio".as_ptr(), 1);
        if lib.is_null() {
            return None;
        }
        let create = NonNull::new(dlsym(lib, c"AudioHardwareCreateProcessTap".as_ptr()))?;
        let destroy = NonNull::new(dlsym(lib, c"AudioHardwareDestroyProcessTap".as_ptr()))?;
        Some((
            std::mem::transmute::<*mut c_void, CreateTap>(create.as_ptr()),
            std::mem::transmute::<*mut c_void, DestroyTap>(destroy.as_ptr()),
        ))
    })
}

// ---------- Propriedades de AudioObject ----------

fn address(selector: AudioObjectPropertySelector) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    }
}

/// Lê uma propriedade de tamanho fixo.
unsafe fn get<T: Copy>(object: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<T> {
    let addr = address(selector);
    let mut value = MaybeUninit::<T>::uninit();
    let mut size = size_of::<T>() as u32;
    let status = AudioObjectGetPropertyData(
        object,
        NonNull::from(&addr),
        0,
        ptr::null(),
        NonNull::from(&mut size),
        NonNull::new_unchecked(value.as_mut_ptr().cast()),
    );
    (status == 0 && size as usize == size_of::<T>()).then(|| value.assume_init())
}

unsafe fn get_ids(object: AudioObjectID, selector: AudioObjectPropertySelector) -> Vec<AudioObjectID> {
    let addr = address(selector);
    let mut size = 0u32;
    if AudioObjectGetPropertyDataSize(object, NonNull::from(&addr), 0, ptr::null(), NonNull::from(&mut size)) != 0 {
        return Vec::new();
    }
    let mut ids = vec![0 as AudioObjectID; size as usize / size_of::<AudioObjectID>()];
    if ids.is_empty() {
        return ids;
    }
    let status = AudioObjectGetPropertyData(
        object,
        NonNull::from(&addr),
        0,
        ptr::null(),
        NonNull::from(&mut size),
        NonNull::new_unchecked(ids.as_mut_ptr().cast()),
    );
    ids.truncate(if status == 0 { size as usize / size_of::<AudioObjectID>() } else { 0 });
    ids
}

/// Propriedades CFString vêm retidas: quem lê libera.
unsafe fn get_string(object: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<String> {
    let raw = NonNull::new(get::<*mut CFString>(object, selector)?)?;
    Some(CFRetained::from_raw(raw).to_string())
}

const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

fn spotify_processes() -> Vec<AudioObjectID> {
    unsafe {
        get_ids(SYSTEM, kAudioHardwarePropertyProcessObjectList)
            .into_iter()
            .filter(|&p| get_string(p, kAudioProcessPropertyBundleID).is_some_and(|b| b.starts_with(BUNDLE_PREFIX)))
            .collect()
    }
}

/// Bundle ids dos processos, fora o próprio app, com saída de áudio aberta (pode estar em silêncio).
/// Helpers de navegador vêm como `com.google.Chrome.helper...`.
pub fn output_bundles() -> Vec<String> {
    let own = std::process::id() as i32;
    unsafe {
        get_ids(SYSTEM, kAudioHardwarePropertyProcessObjectList)
            .into_iter()
            .filter(|&p| {
                get::<u32>(p, kAudioProcessPropertyIsRunningOutput).is_some_and(|r| r != 0)
                    && get::<i32>(p, kAudioProcessPropertyPID).is_some_and(|pid| pid != own)
            })
            .map(|p| get_string(p, kAudioProcessPropertyBundleID).unwrap_or_default())
            .collect()
    }
}

fn system_output_running() -> bool {
    !output_bundles().is_empty()
}

// ---------- Análise (thread de IO, tempo real: sem alocação nem lock) ----------

/// Passa-banda RBJ (ganho 0 dB no centro), forma direta transposta II.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn band_pass(freq: f32, q: f32, rate: f32) -> Self {
        let w = std::f32::consts::TAU * freq.min(rate * 0.45) / rate;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self { b0: alpha / a0, b2: -alpha / a0, a1: -2.0 * w.cos() / a0, a2: (1.0 - alpha) / a0, z1: 0.0, z2: 0.0 }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = -self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Saída da análise, lida pelo emissor.
#[derive(Default)]
struct Levels {
    bands: [AtomicU32; 4],
    /// Quantos blocos de áudio de verdade já chegaram (com permissão negada o tap só entrega zeros);
    /// quem lê compara com a leitura anterior pra saber se ainda há som.
    heard: AtomicU32,
}

struct Analyzer {
    rate: f32,
    filters: [Biquad; 4],
    env: [f32; 4],
    avg: [f32; 4],
    peak: [f32; 4],
    levels: Arc<Levels>,
}

impl Analyzer {
    fn new(rate: f32, levels: Arc<Levels>) -> Self {
        Self {
            rate,
            filters: BANDS.map(|f| Biquad::band_pass(f, BAND_Q, rate)),
            env: [0.0; 4],
            avg: [SILENCE; 4],
            peak: [SILENCE; 4],
            levels,
        }
    }

    fn process(&mut self, buffers: &[AudioBuffer]) {
        let Some(first) = buffers.first() else { return };
        let channels = first.mNumberChannels.max(1) as usize;
        let frames = first.mDataByteSize as usize / size_of::<f32>() / channels;
        if frames == 0 || first.mData.is_null() {
            return;
        }
        let total: usize = buffers.iter().map(|b| b.mNumberChannels as usize).sum();
        let gain = 1.0 / total.max(1) as f32;

        let mut energy = [0.0f32; 4];
        for f in 0..frames {
            // mixdown mono, interleaved ou não
            let mut x = 0.0;
            for b in buffers {
                let ch = b.mNumberChannels as usize;
                if b.mData.is_null() || (f + 1) * ch * size_of::<f32>() > b.mDataByteSize as usize {
                    continue;
                }
                let data = b.mData as *const f32;
                for c in 0..ch {
                    x += unsafe { *data.add(f * ch + c) };
                }
            }
            x *= gain;
            for (e, filter) in energy.iter_mut().zip(&mut self.filters) {
                let y = filter.run(x);
                *e += y * y;
            }
        }

        let dt = frames as f32 / self.rate;
        let attack = 1.0 - (-dt / ATTACK).exp();
        let release = 1.0 - (-dt / RELEASE).exp();
        let smooth = 1.0 - (-dt / AVERAGE).exp();
        let decay = 0.5f32.powf(dt / PEAK_DECAY);
        let db = |ratio: f32| 20.0 * ratio.max(1e-6).log10();
        let mut heard = false;
        for (i, e) in energy.into_iter().enumerate() {
            let rms = (e / frames as f32).sqrt();
            let env = &mut self.env[i];
            *env += (rms - *env) * if rms > *env { attack } else { release };
            let avg = &mut self.avg[i];
            *avg = (*avg + (*env - *avg) * smooth).max(SILENCE);
            let peak = &mut self.peak[i];
            *peak = (*peak * decay).max(*env).max(SILENCE);
            heard |= *env > SILENCE;
            // batida: meio da barra = na média; loudness: encolhe tudo em trecho baixo
            let punch = (0.5 + db(*env / *avg) / (2.0 * PUNCH_DB)).clamp(0.0, 1.0);
            let loud = (1.0 + db(*avg / *peak) / RANGE_DB).clamp(0.0, 1.0);
            let level = if *env > SILENCE { punch * loud } else { 0.0 };
            self.levels.bands[i].store(level.to_bits(), Ordering::Relaxed);
        }
        if heard {
            self.levels.heard.fetch_add(1, Ordering::Relaxed);
        }
    }
}

unsafe extern "C-unwind" fn io_proc(
    _device: AudioObjectID,
    _now: NonNull<AudioTimeStamp>,
    input: NonNull<AudioBufferList>,
    _input_time: NonNull<AudioTimeStamp>,
    _output: NonNull<AudioBufferList>,
    _output_time: NonNull<AudioTimeStamp>,
    client: *mut c_void,
) -> i32 {
    let analyzer = &mut *(client as *mut Analyzer);
    let list = input.as_ptr();
    let count = (*list).mNumberBuffers as usize;
    let buffers = std::slice::from_raw_parts(ptr::addr_of!((*list).mBuffers).cast::<AudioBuffer>(), count);
    analyzer.process(buffers);
    0
}

// ---------- Tap ----------

fn ns(key: &CStr) -> Retained<NSString> {
    NSString::from_str(key.to_str().unwrap_or_default())
}

fn dict(entries: &[(&CStr, &AnyObject)]) -> Retained<NSDictionary<NSString, AnyObject>> {
    let keys: Vec<Retained<NSString>> = entries.iter().map(|(k, _)| ns(k)).collect();
    let keys: Vec<&NSString> = keys.iter().map(|k| &**k).collect();
    let values: Vec<&AnyObject> = entries.iter().map(|(_, v)| *v).collect();
    NSDictionary::from_slices(&keys, &values)
}

/// O que o tap captura.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    Spotify,
    /// Mixagem de todos os apps; o frontend só mostra enquanto houver som audível.
    System,
}

/// Tap + aggregate device privado que o lê. Desfaz tudo no Drop.
struct Tap {
    source: Source,
    tap: AudioObjectID,
    device: AudioObjectID,
    proc_id: AudioDeviceIOProcID,
    analyzer: *mut Analyzer,
    alive: Arc<AtomicBool>,
}

impl Tap {
    fn start(app: &AppHandle, source: Source) -> Option<Self> {
        let (create_tap, destroy_tap) = tap_api()?;
        unsafe {
            let desc = match source {
                Source::Spotify => {
                    let processes = spotify_processes();
                    if processes.is_empty() {
                        return None;
                    }
                    let numbers: Vec<Retained<NSNumber>> = processes.iter().map(|&p| NSNumber::new_u32(p)).collect();
                    CATapDescription::initStereoMixdownOfProcesses(
                        CATapDescription::alloc(),
                        &NSArray::from_retained_slice(&numbers),
                    )
                }
                Source::System => CATapDescription::initStereoGlobalTapButExcludeProcesses(
                    CATapDescription::alloc(),
                    &NSArray::<NSNumber>::new(),
                ),
            };
            desc.setPrivate(true);
            desc.setMuteBehavior(CATapMuteBehavior::Unmuted);

            let mut tap: AudioObjectID = 0;
            if create_tap(&*desc, &mut tap) != 0 || tap == 0 {
                return None;
            }
            let tap_uid = desc.UUID().UUIDString();

            let device = get::<AudioObjectID>(SYSTEM, kAudioHardwarePropertyDefaultOutputDevice);
            let output_uid = device.and_then(|d| get_string(d, kAudioDevicePropertyDeviceUID));
            let format = get::<AudioStreamBasicDescription>(tap, kAudioTapPropertyFormat);
            let (Some(output_uid), Some(format)) = (output_uid, format) else {
                destroy_tap(tap);
                return None;
            };
            let float32 = format.mFormatID == kAudioFormatLinearPCM
                && format.mFormatFlags & kAudioFormatFlagIsFloat != 0
                && format.mBitsPerChannel == 32;
            if !float32 || format.mSampleRate <= 0.0 {
                destroy_tap(tap);
                return None;
            }

            let output_uid = NSString::from_str(&output_uid);
            let sub_device = dict(&[(kAudioSubDeviceUIDKey, &output_uid)]);
            let sub_tap = dict(&[(kAudioSubTapUIDKey, &tap_uid), (kAudioSubTapDriftCompensationKey, &NSNumber::new_bool(true))]);
            let aggregate = dict(&[
                (kAudioAggregateDeviceNameKey, &NSString::from_str("Dynamic Lite Equalizer")),
                (kAudioAggregateDeviceUIDKey, &desc.UUID().UUIDString().stringByAppendingString(&NSString::from_str("-agg"))),
                (kAudioAggregateDeviceMainSubDeviceKey, &output_uid),
                (kAudioAggregateDeviceIsPrivateKey, &NSNumber::new_bool(true)),
                (kAudioAggregateDeviceIsStackedKey, &NSNumber::new_bool(false)),
                (kAudioAggregateDeviceTapAutoStartKey, &NSNumber::new_bool(true)),
                (kAudioAggregateDeviceSubDeviceListKey, &NSArray::from_retained_slice(&[sub_device])),
                (kAudioAggregateDeviceTapListKey, &NSArray::from_retained_slice(&[sub_tap])),
            ]);
            // NSDictionary e CFDictionary são toll-free bridged
            let aggregate = &*(Retained::as_ptr(&aggregate) as *const CFDictionary);

            let mut device: AudioObjectID = 0;
            if AudioHardwareCreateAggregateDevice(aggregate, NonNull::from(&mut device)) != 0 {
                destroy_tap(tap);
                return None;
            }

            let levels = Arc::new(Levels::default());
            let analyzer = Box::into_raw(Box::new(Analyzer::new(format.mSampleRate as f32, levels.clone())));
            let mut proc_id: AudioDeviceIOProcID = None;
            let created = AudioDeviceCreateIOProcID(device, Some(io_proc), analyzer.cast(), NonNull::from(&mut proc_id));
            let alive = Arc::new(AtomicBool::new(true));
            // Drop desfaz o que já foi criado se o start falhar
            let this = Self { source, tap, device, proc_id, analyzer, alive: alive.clone() };
            if created != 0 || proc_id.is_none() || AudioDeviceStart(device, proc_id) != 0 {
                return None;
            }
            spawn_emitter(app.clone(), levels, alive, source);
            Some(this)
        }
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
        if self.source == Source::System {
            SYSTEM_ACTIVE.store(false, Ordering::Relaxed);
        }
        unsafe {
            if self.proc_id.is_some() {
                // síncrono: depois disso o io_proc não roda mais
                AudioDeviceStop(self.device, self.proc_id);
                AudioDeviceDestroyIOProcID(self.device, self.proc_id);
            }
            AudioHardwareDestroyAggregateDevice(self.device);
            if let Some((_, destroy_tap)) = tap_api() {
                destroy_tap(self.tap);
            }
            drop(Box::from_raw(self.analyzer));
        }
    }
}

#[tauri::command]
pub fn equalizer_fps() -> u64 {
    FPS
}

/// Há som audível no sistema agora (lido por `equalizer_system_active`; o evento cobre as mudanças).
static SYSTEM_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn equalizer_system_active() -> bool {
    system_active()
}

pub fn system_active() -> bool {
    SYSTEM_ACTIVE.load(Ordering::Relaxed)
}

fn spawn_emitter(app: AppHandle, levels: Arc<Levels>, alive: Arc<AtomicBool>, source: Source) {
    thread::spawn(move || {
        let mut sent = [f32::NAN; 4];
        let mut heard = 0;
        let mut silent_since = Instant::now();
        let mut audible = false;
        while alive.load(Ordering::Relaxed) {
            thread::sleep(FRAME);
            if !alive.load(Ordering::Relaxed) {
                break;
            }
            let count = levels.heard.load(Ordering::Relaxed);
            if count != heard {
                heard = count;
                silent_since = Instant::now();
            }
            if source == Source::System {
                let now = count > 0 && silent_since.elapsed() < SYSTEM_HOLD;
                if now != audible {
                    audible = now;
                    SYSTEM_ACTIVE.store(now, Ordering::Relaxed);
                    let _ = app.emit(SYSTEM_EVENT, now);
                }
            }
            if count > 0 {
                let bands = levels.bands.each_ref().map(|b| f32::from_bits(b.load(Ordering::Relaxed)));
                // NaN inicial: o primeiro quadro sempre sai
                if bands.iter().zip(&sent).any(|(b, s)| !((b - s).abs() < MIN_DELTA)) {
                    sent = bands;
                    let _ = app.emit(LEVELS_EVENT, bands);
                }
            }
        }
        if audible {
            let _ = app.emit(SYSTEM_EVENT, false);
        }
    });
}

pub enum Msg {
    /// O Spotify começou ou parou de tocar.
    Spotify(bool),
    /// Os Settings mudaram.
    Refresh,
}

/// Canal do controlador, guardado no estado do Tauri pra os Settings avisarem.
pub struct Control(Sender<Msg>);

impl Control {
    pub fn refresh(&self) {
        let _ = self.0.send(Msg::Refresh);
    }
}

/// Fonte que deve estar capturada agora: o Spotify tocando manda; senão, o sistema (se ligado e com saída aberta).
fn wanted(spotify: bool, system: bool) -> Option<Source> {
    if spotify {
        Some(Source::Spotify)
    } else if system && system_output_running() {
        Some(Source::System)
    } else {
        None
    }
}

/// Liga o tap enquanto há o que mostrar; sem áudio, desliga (some o indicador de gravação).
/// Thread própria: criar o tap pode bloquear (prompt de permissão) e não deve travar o watcher.
pub fn spawn(app: AppHandle) -> Sender<Msg> {
    let (tx, rx) = mpsc::channel();
    app.manage(Control(tx.clone()));
    thread::spawn(move || {
        let settings = app.state::<SharedSettings>();
        let mut tap: Option<Tap> = None;
        let mut spotify = false;
        loop {
            let system = settings.lock().unwrap().system_audio;
            // com o sistema ligado e nada tocando, olha a cada segundo se algum app começou
            let wait = if system { POLL } else { RETRY };
            match rx.recv_timeout(wait) {
                Ok(Msg::Spotify(p)) => spotify = p,
                Ok(Msg::Refresh) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            while let Ok(msg) = rx.try_recv() {
                if let Msg::Spotify(p) = msg {
                    spotify = p;
                }
            }
            let system = settings.lock().unwrap().system_audio;
            match wanted(spotify, system) {
                None => tap = None,
                Some(source) if tap.as_ref().is_none_or(|t| t.source != source) => {
                    // o tap antigo sai antes do novo entrar
                    drop(tap.take());
                    tap = Tap::start(&app, source);
                }
                Some(_) => {}
            }
        }
    });
    tx
}

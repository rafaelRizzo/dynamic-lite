//! Equalizer ao vivo: captura o áudio que sai pela caixa de som padrão (WASAPI loopback, sem pedir permissão e
//! sem indicador de gravação) e reduz a 4 bandas de energia, emitidas pro frontend. O loopback pega o mix inteiro
//! do sistema, então com o Spotify tocando a barra segue o Spotify (e qualquer outro som junto).
//! Sem áudio saindo, nada é emitido e o frontend mantém a animação CSS.

use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator,
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

use crate::settings::SharedSettings;
use crate::win;

pub const LEVELS_EVENT: &str = "equalizer://levels";
/// `bool`: há áudio audível no sistema (só com a fonte Sistema).
pub const SYSTEM_EVENT: &str = "equalizer://system";
/// Quadros por segundo, do envio dos níveis e do desenho no frontend (lido por `equalizer_fps`).
const FPS: u64 = 24;
const FRAME: Duration = Duration::from_millis(1000 / FPS);
/// Variação mínima (0-1) em alguma barra pra valer um novo evento; abaixo disso o olho não vê.
const MIN_DELTA: f32 = 0.015;
/// Tocando sem captura (nenhum dispositivo de saída, device ocupado): tenta de novo.
const RETRY: Duration = Duration::from_secs(5);
/// Com a fonte Sistema ligada, de quanto em quanto tempo se olha se a captura ainda está viva.
const POLL: Duration = Duration::from_secs(1);
/// Sem sinal por esse tempo, o áudio do sistema conta como parado (pausas e silêncios curtos não piscam a Island).
const SYSTEM_HOLD: Duration = Duration::from_secs(2);
/// Quanto esperar a captura abrir antes de desistir.
const START_TIMEOUT: Duration = Duration::from_secs(3);
/// Intervalo de leitura do buffer de loopback.
const READ_EVERY: Duration = Duration::from_millis(10);

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
/// ~-80 dBFS: abaixo disso é silêncio.
const SILENCE: f32 = 1e-4;

// ---------- Análise ----------

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
    /// Quantos blocos de áudio de verdade já chegaram; quem lê compara com a leitura anterior pra saber se ainda há som.
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

    /// `mono`: o bloco já misturado num canal só.
    fn process(&mut self, mono: &[f32]) {
        let frames = mono.len();
        if frames == 0 {
            return;
        }
        let mut energy = [0.0f32; 4];
        for &x in mono {
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

// ---------- Captura (WASAPI loopback) ----------

#[derive(Clone, Copy, PartialEq)]
enum Source {
    Spotify,
    System,
}

#[derive(Clone, Copy, PartialEq)]
enum Sample {
    F32,
    I16,
}

/// Formato de mix do dispositivo: no modo compartilhado é quase sempre float de 32 bits.
fn sample_kind(format: &WAVEFORMATEX) -> Option<Sample> {
    const PCM: u16 = 1;
    const FLOAT: u16 = 3;
    const EXTENSIBLE: u16 = 0xFFFE;
    let (tag, bits) = (format.wFormatTag, format.wBitsPerSample);
    match (tag, bits) {
        (FLOAT, 32) => Some(Sample::F32),
        (PCM, 16) => Some(Sample::I16),
        // extensível: o subformato é float nos 32 bits e PCM nos 16 (o único jeito de o mix vir assim)
        (EXTENSIBLE, 32) => Some(Sample::F32),
        (EXTENSIBLE, 16) => Some(Sample::I16),
        _ => None,
    }
}

/// Mistura `frames` quadros intercalados de `channels` canais num canal só, em `out`.
///
/// # Safety
/// `data` aponta para `frames * channels` amostras do tipo `kind`.
unsafe fn mix_down(data: *const u8, frames: usize, channels: usize, kind: Sample, out: &mut Vec<f32>) {
    out.clear();
    let gain = 1.0 / channels as f32;
    match kind {
        Sample::F32 => {
            let samples = std::slice::from_raw_parts(data.cast::<f32>(), frames * channels);
            out.extend(samples.chunks_exact(channels).map(|f| f.iter().sum::<f32>() * gain));
        }
        Sample::I16 => {
            let samples = std::slice::from_raw_parts(data.cast::<i16>(), frames * channels);
            out.extend(samples.chunks_exact(channels).map(|f| f.iter().map(|&s| f32::from(s) / 32768.0).sum::<f32>() * gain));
        }
    }
}

/// Abre o loopback da saída padrão e alimenta o `Analyzer` até `alive` cair ou o dispositivo sumir.
fn capture(levels: Arc<Levels>, alive: Arc<AtomicBool>, ready: &Sender<bool>) -> windows::core::Result<()> {
    win::com_init();
    // SAFETY: chamadas COM; os ponteiros do WASAPI só são lidos entre GetBuffer e ReleaseBuffer
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let format = client.GetMixFormat()?;
        let (rate, channels, kind) = {
            let f = &*format;
            (f.nSamplesPerSec, usize::from(f.nChannels), sample_kind(f))
        };
        let init = client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 1_000_000, 0, format, None);
        CoTaskMemFree(Some(format.cast()));
        init?;
        let Some(kind) = kind.filter(|_| channels > 0) else { return Err(windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)) };

        let capture: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let _ = ready.send(true);

        let mut analyzer = Analyzer::new(rate as f32, levels);
        let mut mono: Vec<f32> = Vec::with_capacity(rate as usize / 20);
        // 10 ms de silêncio, pra os envelopes caírem quando o loopback para de entregar pacotes
        let quiet = vec![0.0f32; (rate / 100) as usize];
        while alive.load(Ordering::Relaxed) {
            thread::sleep(READ_EVERY);
            let mut got = false;
            while capture.GetNextPacketSize()? > 0 {
                let mut data: *mut u8 = ptr::null_mut();
                let (mut frames, mut flags) = (0u32, 0u32);
                capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                if flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0 || data.is_null() {
                    mono.clear();
                    mono.resize(frames as usize, 0.0);
                } else {
                    mix_down(data, frames as usize, channels, kind, &mut mono);
                }
                analyzer.process(&mono);
                capture.ReleaseBuffer(frames)?;
                got = true;
            }
            if !got {
                analyzer.process(&quiet);
            }
        }
        let _ = client.Stop();
    }
    Ok(())
}

struct Tap {
    source: Source,
    alive: Arc<AtomicBool>,
    /// A thread de captura terminou (dispositivo trocado ou removido): é preciso abrir de novo.
    dead: Arc<AtomicBool>,
}

impl Tap {
    fn start(app: &AppHandle, source: Source) -> Option<Self> {
        let levels = Arc::new(Levels::default());
        let alive = Arc::new(AtomicBool::new(true));
        let dead = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::channel();
        {
            let (levels, alive, dead) = (levels.clone(), alive.clone(), dead.clone());
            thread::spawn(move || {
                if capture(levels, alive, &ready_tx).is_err() {
                    let _ = ready_tx.send(false);
                }
                dead.store(true, Ordering::Relaxed);
            });
        }
        if !ready_rx.recv_timeout(START_TIMEOUT).unwrap_or(false) {
            alive.store(false, Ordering::Relaxed);
            return None;
        }
        spawn_emitter(app.clone(), levels, alive.clone(), source);
        Some(Self { source, alive, dead })
    }

    fn is_dead(&self) -> bool {
        self.dead.load(Ordering::Relaxed)
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
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
            SYSTEM_ACTIVE.store(false, Ordering::Relaxed);
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

/// Fonte que deve estar capturada agora: o Spotify tocando manda; senão, o sistema (se ligado).
fn wanted(spotify: bool, system: bool) -> Option<Source> {
    if spotify {
        Some(Source::Spotify)
    } else if system {
        Some(Source::System)
    } else {
        None
    }
}

/// Liga a captura enquanto há o que mostrar; sem áudio, desliga.
/// Thread própria: abrir o dispositivo pode demorar e não deve travar o watcher.
pub fn spawn(app: AppHandle) -> Sender<Msg> {
    let (tx, rx) = mpsc::channel();
    app.manage(Control(tx.clone()));
    thread::spawn(move || {
        let settings = app.state::<SharedSettings>();
        let mut tap: Option<Tap> = None;
        let mut spotify = false;
        loop {
            let system = settings.lock().unwrap().system_audio;
            // com o sistema ligado e nada tocando, olha a cada segundo se a captura caiu
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
            if tap.as_ref().is_some_and(Tap::is_dead) {
                tap = None;
            }
            let system = settings.lock().unwrap().system_audio;
            match wanted(spotify, system) {
                None => tap = None,
                Some(source) if tap.as_ref().is_none_or(|t| t.source != source) => {
                    // a captura antiga sai antes da nova entrar
                    drop(tap.take());
                    tap = Tap::start(&app, source);
                }
                Some(_) => {}
            }
        }
    });
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: f32, frames: usize) -> Vec<f32> {
        (0..frames).map(|i| (std::f32::consts::TAU * freq * i as f32 / rate).sin() * 0.5).collect()
    }

    #[test]
    fn silence_is_not_heard() {
        let levels = Arc::new(Levels::default());
        let mut analyzer = Analyzer::new(48_000.0, levels.clone());
        analyzer.process(&vec![0.0; 480]);
        assert_eq!(levels.heard.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn tone_is_heard_in_its_band() {
        let levels = Arc::new(Levels::default());
        let mut analyzer = Analyzer::new(48_000.0, levels.clone());
        for _ in 0..20 {
            analyzer.process(&sine(80.0, 48_000.0, 480));
        }
        assert!(levels.heard.load(Ordering::Relaxed) > 0);
        assert!(analyzer.env[0] > analyzer.env[3] * 10.0);
    }

    #[test]
    fn mix_down_averages_channels() {
        let stereo = [1.0f32, 0.0, 0.5, 0.5];
        let mut out = vec![];
        unsafe { mix_down(stereo.as_ptr().cast(), 2, 2, Sample::F32, &mut out) };
        assert_eq!(out, vec![0.5, 0.5]);
    }
}

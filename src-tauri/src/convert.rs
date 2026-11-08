//! Conversor de áudio nativo (sem ffmpeg): decodifica com Symphonia (MP3, WAV, FLAC, OGG, AAC/M4A...) e com a libgsm
//! (.gsm cru e WAV49), ajusta canais e taxa, e grava em WAV, G.711 (µ-law e A-law), GSM ou MP3.
//! Tudo roda numa thread de trabalho e avisa o progresso por evento. Nada sai do computador.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tauri::{AppHandle, Emitter};

pub const PROGRESS_EVENT: &str = "convert://progress";

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Format {
    /// WAV PCM de 16 bits
    Wav,
    /// WAV G.711 µ-law (8 bits)
    WavUlaw,
    /// WAV G.711 A-law (8 bits)
    WavAlaw,
    /// GSM 06.10 cru (.gsm, o que o Asterisk usa)
    Gsm,
    /// GSM 06.10 dentro de WAV (WAV49)
    WavGsm,
    Mp3,
}

impl Format {
    fn extension(self) -> &'static str {
        match self {
            Format::Wav | Format::WavUlaw | Format::WavAlaw | Format::WavGsm => "wav",
            Format::Gsm => "gsm",
            Format::Mp3 => "mp3",
        }
    }

    /// Formatos de telefonia só existem em 8 kHz mono.
    fn fixed(self) -> Option<(u32, u8)> {
        matches!(self, Format::WavUlaw | Format::WavAlaw | Format::Gsm | Format::WavGsm).then_some((8000, 1))
    }
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub format: Format,
    /// Hz; sem valor, mantém a do arquivo
    pub sample_rate: Option<u32>,
    /// 1 ou 2; sem valor, mantém os do arquivo
    pub channels: Option<u8>,
    /// kbps, só no MP3
    pub bitrate: Option<u32>,
    /// leva o pico a -1 dBFS
    pub normalize: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Converted {
    pub path: String,
    pub bytes: u64,
    pub seconds: f64,
}

/// Áudio em amostras de ponto flutuante, um vetor por canal.
struct Pcm {
    rate: u32,
    channels: Vec<Vec<f32>>,
}

impl Pcm {
    fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }
}

// ---------- Decodificação ----------

fn read_wav_format_tag(bytes: &[u8]) -> Option<u16> {
    if bytes.len() < 22 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        if &bytes[at..at + 4] == b"fmt " && at + 10 <= bytes.len() {
            return Some(u16::from_le_bytes(bytes[at + 8..at + 10].try_into().ok()?));
        }
        at += 8 + size + (size & 1);
    }
    None
}

fn wav_data_chunk(bytes: &[u8]) -> Option<&[u8]> {
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        if &bytes[at..at + 4] == b"data" {
            let end = (at + 8 + size).min(bytes.len());
            return Some(&bytes[at + 8..end]);
        }
        at += 8 + size + (size & 1);
    }
    None
}

struct Gsm(gsm_sys::Gsm);

impl Gsm {
    fn new() -> Result<Self, String> {
        // SAFETY: `gsm_create` devolve um estado novo ou nulo
        let handle = unsafe { gsm_sys::gsm_create() };
        if handle.is_null() { Err("Não consegui iniciar o codec GSM".into()) } else { Ok(Gsm(handle)) }
    }

    /// Um quadro GSM 06.10 de 33 bytes em 160 amostras.
    fn decode_frame(&self, frame: &[u8; 33]) -> Result<[i16; 160], String> {
        let mut data = *frame;
        let mut pcm = [0i16; 160];
        // SAFETY: `data` tem o quadro inteiro e `pcm` cabe as 160 amostras
        if unsafe { gsm_sys::gsm_decode(self.0, data.as_mut_ptr(), pcm.as_mut_ptr()) } != 0 {
            return Err("Quadro GSM inválido".into());
        }
        Ok(pcm)
    }

    fn encode_frame(&self, pcm: &[i16; 160]) -> [u8; 33] {
        let (mut input, mut frame) = (*pcm, [0u8; 33]);
        // SAFETY: lê 160 amostras e escreve 33 bytes
        unsafe { gsm_sys::gsm_encode(self.0, input.as_mut_ptr(), frame.as_mut_ptr()) };
        frame
    }

    /// Os 76 parâmetros do quadro (8 LAR, depois 4 × [Nc, bc, Mc, xmax, 13 × xmc]).
    fn explode(&self, frame: &[u8; 33]) -> [i16; 76] {
        let (mut data, mut params) = (*frame, [0i16; 76]);
        // SAFETY: `data` tem 33 bytes e `params` cabe os 76 parâmetros
        unsafe { gsm_sys::gsm_explode(self.0, data.as_mut_ptr(), params.as_mut_ptr()) };
        params
    }

    fn implode(&self, params: &[i16; 76]) -> [u8; 33] {
        let (mut input, mut frame) = (*params, [0u8; 33]);
        // SAFETY: lê 76 parâmetros e escreve 33 bytes
        unsafe { gsm_sys::gsm_implode(self.0, input.as_mut_ptr(), frame.as_mut_ptr()) };
        frame
    }
}

impl Drop for Gsm {
    fn drop(&mut self) {
        // SAFETY: o estado veio de `gsm_create` e só é liberado aqui
        unsafe { gsm_sys::gsm_destroy(self.0) }
    }
}

/// Largura em bits de cada um dos 76 parâmetros, na ordem do WAV49 (a mesma do quadro cru).
fn wav49_widths() -> [u32; 76] {
    let mut w = [0u32; 76];
    w[..8].copy_from_slice(&[6, 6, 5, 5, 4, 4, 3, 3]);
    for sub in 0..4 {
        let at = 8 + sub * 17;
        w[at..at + 4].copy_from_slice(&[7, 2, 2, 6]);
        w[at + 4..at + 17].fill(3);
    }
    w
}

/// WAV49: os parâmetros de dois quadros em sequência, bit a bit, do menos significativo pro mais (65 bytes).
fn pack_wav49(a: &[i16; 76], b: &[i16; 76]) -> [u8; 65] {
    let widths = wav49_widths();
    let mut out = [0u8; 65];
    let mut bit = 0usize;
    for params in [a, b] {
        for (&value, &width) in params.iter().zip(&widths) {
            for k in 0..width {
                if (value >> k) & 1 == 1 {
                    out[bit / 8] |= 1 << (bit % 8);
                }
                bit += 1;
            }
        }
    }
    out
}

fn unpack_wav49(block: &[u8]) -> [[i16; 76]; 2] {
    let widths = wav49_widths();
    let mut out = [[0i16; 76]; 2];
    let mut bit = 0usize;
    for params in &mut out {
        for (value, &width) in params.iter_mut().zip(&widths) {
            let mut v = 0i16;
            for k in 0..width {
                if (block[bit / 8] >> (bit % 8)) & 1 == 1 {
                    v |= 1 << k;
                }
                bit += 1;
            }
            *value = v;
        }
    }
    out
}

/// GSM 06.10 cru (33 bytes por quadro de 160 amostras) ou WAV49 (65 bytes por 2 quadros).
fn decode_gsm(frames: &[u8], wav49: bool) -> Result<Vec<i16>, String> {
    let gsm = Gsm::new()?;
    let mut out = Vec::new();
    if wav49 {
        for block in frames.chunks_exact(65) {
            for params in unpack_wav49(block) {
                out.extend_from_slice(&gsm.decode_frame(&gsm.implode(&params))?);
            }
        }
    } else {
        for chunk in frames.chunks_exact(33) {
            out.extend_from_slice(&gsm.decode_frame(chunk.try_into().expect("33 bytes"))?);
        }
    }
    Ok(out)
}

fn pcm_from_i16(samples: &[i16], rate: u32) -> Pcm {
    Pcm { rate, channels: vec![samples.iter().map(|&s| f32::from(s) / 32768.0).collect()] }
}

fn decode_symphonia(path: &Path, progress: &mut dyn FnMut(f32)) -> Result<Pcm, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| "Formato de áudio não reconhecido".to_string())?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("O arquivo não tem áudio")?
        .clone();
    let total = track.codec_params.n_frames.unwrap_or(0);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| "Não sei decodificar este áudio".to_string())?;

    let mut rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut channels: Vec<Vec<f32>> = Vec::new();
    let mut buffer: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != track.id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                rate = spec.rate;
                let count = spec.channels.count();
                if channels.is_empty() {
                    channels = vec![Vec::new(); count];
                }
                let buf = buffer.get_or_insert_with(|| SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                buf.copy_interleaved_ref(decoded);
                for frame in buf.samples().chunks_exact(count) {
                    for (c, &s) in channels.iter_mut().zip(frame) {
                        c.push(s);
                    }
                }
                if total > 0 {
                    progress((channels[0].len() as f32 / total as f32).min(1.0));
                }
            }
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    if channels.is_empty() || rate == 0 || channels[0].is_empty() {
        return Err("Não encontrei áudio no arquivo".into());
    }
    Ok(Pcm { rate, channels })
}

fn decode(path: &Path, progress: &mut dyn FnMut(f32)) -> Result<Pcm, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if ext == "gsm" {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        return Ok(pcm_from_i16(&decode_gsm(&bytes, false)?, 8000));
    }
    if ext == "wav" || ext == "wav49" {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        if read_wav_format_tag(&bytes) == Some(0x31) {
            let data = wav_data_chunk(&bytes).ok_or("WAV sem dados")?;
            return Ok(pcm_from_i16(&decode_gsm(data, true)?, 8000));
        }
    }
    decode_symphonia(path, progress)
}

// ---------- Processamento ----------

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Passa-baixa por sinc com janela Blackman, em fases pré-calculadas (resampler polifásico).
/// O corte fica um pouco abaixo da frequência de Nyquist de saída, pra não deixar aliasing.
fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    const ZEROS: f64 = 12.0;
    const MAX_PHASES: u64 = 8192;
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let g = gcd(u64::from(from), u64::from(to));
    let (l, m) = (u64::from(to) / g, u64::from(from) / g);
    let phases = l.min(MAX_PHASES) as usize;
    let fc = 0.5 * (f64::from(to) / f64::from(from)).min(1.0) * 0.96;
    let half = ZEROS / (2.0 * fc);
    let radius = half.ceil() as isize;
    let taps = (2 * radius) as usize;

    let kernels: Vec<Vec<f32>> = (0..phases)
        .map(|p| {
            let frac = p as f64 / phases as f64;
            let mut k: Vec<f64> = (0..taps)
                .map(|j| {
                    let x = (j as isize - radius + 1) as f64 - frac;
                    if x.abs() >= half {
                        return 0.0;
                    }
                    let a = std::f64::consts::PI * 2.0 * fc * x;
                    let sinc = if a.abs() < 1e-9 { 1.0 } else { a.sin() / a };
                    let t = x / half;
                    let window = 0.42 + 0.5 * (std::f64::consts::PI * t).cos() + 0.08 * (2.0 * std::f64::consts::PI * t).cos();
                    2.0 * fc * sinc * window
                })
                .collect();
            let sum: f64 = k.iter().sum();
            if sum.abs() > 1e-12 {
                k.iter_mut().for_each(|v| *v /= sum);
            }
            k.into_iter().map(|v| v as f32).collect()
        })
        .collect();

    let out_len = (input.len() as u64 * l).div_ceil(m) as usize;
    let mut out = Vec::with_capacity(out_len);
    let len = input.len() as isize;
    for n in 0..out_len as u64 {
        let num = n * m;
        let mut int = (num / l) as isize;
        let rem = num % l;
        let mut phase = ((rem as f64 / l as f64) * phases as f64).round() as usize;
        if phase >= phases {
            phase = 0;
            int += 1;
        }
        let kernel = &kernels[phase];
        let start = int - radius + 1;
        let mut acc = 0.0f32;
        if start >= 0 && start + taps as isize <= len {
            let window = &input[start as usize..start as usize + taps];
            acc = window.iter().zip(kernel).map(|(x, k)| x * k).sum();
        } else {
            for (j, k) in kernel.iter().enumerate() {
                let idx = start + j as isize;
                if idx >= 0 && idx < len {
                    acc += input[idx as usize] * k;
                }
            }
        }
        out.push(acc);
    }
    out
}

/// Ajusta o número de canais: mono mistura todos; estéreo duplica o mono ou fica com os dois primeiros.
fn set_channels(channels: Vec<Vec<f32>>, target: usize) -> Vec<Vec<f32>> {
    match (channels.len(), target) {
        (n, t) if n == t => channels,
        (_, 1) => {
            let n = channels.len() as f32;
            let frames = channels[0].len();
            vec![(0..frames).map(|i| channels.iter().map(|c| c[i]).sum::<f32>() / n).collect()]
        }
        (1, 2) => vec![channels[0].clone(), channels[0].clone()],
        (_, 2) => channels.into_iter().take(2).collect(),
        _ => channels,
    }
}

fn to_i16(channels: &[Vec<f32>]) -> Vec<i16> {
    let frames = channels.first().map_or(0, Vec::len);
    let mut out = Vec::with_capacity(frames * channels.len());
    for i in 0..frames {
        for c in channels {
            out.push((c[i] * 32767.0).round().clamp(-32768.0, 32767.0) as i16);
        }
    }
    out
}

// ---------- Codificação ----------

fn ulaw(sample: i16) -> u8 {
    const BIAS: i32 = 0x84;
    const CLIP: i32 = 32635;
    let mut s = i32::from(sample);
    let sign = if s < 0 {
        s = -s;
        0x80
    } else {
        0
    };
    s = s.min(CLIP) + BIAS;
    let mut exp = 7;
    let mut mask = 0x4000;
    while exp > 0 && s & mask == 0 {
        exp -= 1;
        mask >>= 1;
    }
    let mantissa = (s >> (exp + 3)) & 0x0F;
    !(sign | (exp << 4) | mantissa) as u8
}

fn alaw(sample: i16) -> u8 {
    const SEG_END: [i32; 8] = [0x1F, 0x3F, 0x7F, 0xFF, 0x1FF, 0x3FF, 0x7FF, 0xFFF];
    let mut pcm = i32::from(sample) >> 3;
    let mask = if pcm >= 0 {
        0xD5
    } else {
        pcm = -pcm - 1;
        0x55
    };
    let Some(seg) = SEG_END.iter().position(|&end| pcm <= end) else { return (0x7F ^ mask) as u8 };
    let mut value = (seg as i32) << 4;
    value |= if seg < 2 { (pcm >> 1) & 0x0F } else { (pcm >> seg) & 0x0F };
    (value ^ mask) as u8
}

fn riff(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let body: usize = chunks.iter().map(|(_, d)| 8 + d.len() + (d.len() & 1)).sum();
    let mut out = Vec::with_capacity(12 + body);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((4 + body) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    for (id, data) in chunks {
        out.extend_from_slice(*id);
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        if data.len() & 1 == 1 {
            out.push(0);
        }
    }
    out
}

fn fmt_chunk(tag: u16, channels: u16, rate: u32, byte_rate: u32, align: u16, bits: u16, extra: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&tag.to_le_bytes());
    f.extend_from_slice(&channels.to_le_bytes());
    f.extend_from_slice(&rate.to_le_bytes());
    f.extend_from_slice(&byte_rate.to_le_bytes());
    f.extend_from_slice(&align.to_le_bytes());
    f.extend_from_slice(&bits.to_le_bytes());
    if tag != 1 {
        f.extend_from_slice(&(extra.len() as u16).to_le_bytes());
        f.extend_from_slice(extra);
    }
    f
}

fn wav_pcm16(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    riff(&[(b"fmt ", fmt_chunk(1, channels, rate, rate * u32::from(channels) * 2, channels * 2, 16, &[])), (b"data", data)])
}

fn wav_g711(samples: &[i16], tag: u16, encode: fn(i16) -> u8) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().map(|&s| encode(s)).collect();
    let fact = (samples.len() as u32).to_le_bytes().to_vec();
    riff(&[(b"fmt ", fmt_chunk(tag, 1, 8000, 8000, 1, 8, &[])), (b"fact", fact), (b"data", data)])
}

fn encode_gsm(samples: &[i16], wav49: bool) -> Result<Vec<u8>, String> {
    let gsm = Gsm::new()?;
    let mut frames = samples.chunks(160).map(|chunk| {
        let mut block = [0i16; 160];
        block[..chunk.len()].copy_from_slice(chunk);
        gsm.encode_frame(&block)
    });
    let mut out = Vec::with_capacity(samples.len().div_ceil(160) * 33);
    if wav49 {
        while let Some(first) = frames.next() {
            // sobrou um quadro: o bloco fecha com silêncio. Só aqui, porque o codificador guarda estado
            // e codificar silêncio antes mudaria todos os quadros seguintes
            let second = frames.next().unwrap_or_else(|| gsm.encode_frame(&[0i16; 160]));
            out.extend_from_slice(&pack_wav49(&gsm.explode(&first), &gsm.explode(&second)));
        }
    } else {
        frames.for_each(|f| out.extend_from_slice(&f));
    }
    Ok(out)
}

fn wav_gsm(samples: &[i16]) -> Result<Vec<u8>, String> {
    let data = encode_gsm(samples, true)?;
    let mut extra = Vec::new();
    extra.extend_from_slice(&320u16.to_le_bytes());
    let fact = (samples.len() as u32).to_le_bytes().to_vec();
    Ok(riff(&[(b"fmt ", fmt_chunk(0x31, 1, 8000, 1625, 65, 0, &extra)), (b"fact", fact), (b"data", data)]))
}

fn bitrate(kbps: u32) -> mp3lame_encoder::Bitrate {
    use mp3lame_encoder::Bitrate::*;
    match kbps {
        0..=8 => Kbps8,
        9..=16 => Kbps16,
        17..=24 => Kbps24,
        25..=32 => Kbps32,
        33..=40 => Kbps40,
        41..=48 => Kbps48,
        49..=64 => Kbps64,
        65..=80 => Kbps80,
        81..=96 => Kbps96,
        97..=112 => Kbps112,
        113..=128 => Kbps128,
        129..=160 => Kbps160,
        161..=192 => Kbps192,
        193..=224 => Kbps224,
        225..=256 => Kbps256,
        _ => Kbps320,
    }
}

/// Taxas que o MP3 aceita; qualquer outra vai pra mais próxima.
fn mp3_rate(rate: u32) -> u32 {
    const RATES: [u32; 9] = [8000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000];
    RATES.into_iter().min_by_key(|r| r.abs_diff(rate)).unwrap_or(44_100)
}

fn encode_mp3(samples: &[i16], channels: u8, rate: u32, kbps: u32) -> Result<Vec<u8>, String> {
    use mp3lame_encoder::{Builder, FlushNoGap, InterleavedPcm, MonoPcm, Quality};
    let err = |what: &str| format!("Falha no MP3: {what}");
    let mut builder = Builder::new().ok_or_else(|| err("codec indisponível"))?;
    builder.set_num_channels(channels).map_err(|_| err("canais"))?;
    builder.set_sample_rate(rate).map_err(|_| err("taxa de amostragem"))?;
    builder.set_brate(bitrate(kbps)).map_err(|_| err("taxa de bits"))?;
    builder.set_quality(Quality::Good).map_err(|_| err("qualidade"))?;
    let mut encoder = builder.build().map_err(|_| err("inicialização"))?;
    let mut out = Vec::new();
    // blocos pequenos mantêm o buffer de saída limitado
    for chunk in samples.chunks(1152 * 64 * usize::from(channels)) {
        // `encode_to_vec` escreve só na capacidade livre: sem reservar, o LAME recebe um buffer de tamanho zero
        out.reserve(mp3lame_encoder::max_required_buffer_size(chunk.len() / usize::from(channels)));
        if channels == 1 {
            encoder.encode_to_vec(MonoPcm(chunk), &mut out).map_err(|_| err("codificação"))?;
        } else {
            encoder.encode_to_vec(InterleavedPcm(chunk), &mut out).map_err(|_| err("codificação"))?;
        }
    }
    out.reserve(7200);
    encoder.flush_to_vec::<FlushNoGap>(&mut out).map_err(|_| err("finalização"))?;
    Ok(out)
}

// ---------- Orquestração ----------

/// `pasta/nome_sufixo.ext`, com " (2)", " (3)"... se já existir: nunca sobrescreve.
fn free_path(dir: &Path, stem: &str, suffix: &str, ext: &str) -> PathBuf {
    let base = if suffix.is_empty() { stem.to_owned() } else { format!("{stem}_{suffix}") };
    let first = dir.join(format!("{base}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{base} ({n}).{ext}")))
        .find(|p| !p.exists())
        .expect("sequência infinita")
}

pub fn convert(input: &Path, out_dir: &Path, suffix: &str, opts: &Options, progress: &mut dyn FnMut(f32)) -> Result<Converted, String> {
    let (decode_part, process_part) = (0.4f32, 0.3f32);
    let pcm = decode(input, &mut |f| progress(f * decode_part))?;
    let seconds = pcm.frames() as f64 / f64::from(pcm.rate);

    let (mut rate, channels) = match opts.format.fixed() {
        Some(fixed) => fixed,
        None => (opts.sample_rate.unwrap_or(pcm.rate).clamp(8000, 192_000), opts.channels.unwrap_or(pcm.channels.len() as u8).clamp(1, 2)),
    };
    if opts.format == Format::Mp3 {
        rate = mp3_rate(rate);
    }
    let mut planar = set_channels(pcm.channels, usize::from(channels));
    let count = planar.len();
    for (i, c) in planar.iter_mut().enumerate() {
        *c = resample(c, pcm.rate, rate);
        progress(decode_part + process_part * (i + 1) as f32 / count as f32);
    }
    if opts.normalize {
        let peak = planar.iter().flatten().fold(0.0f32, |m, s| m.max(s.abs()));
        if peak > 0.0 {
            let gain = 0.891 / peak;
            planar.iter_mut().flatten().for_each(|s| *s *= gain);
        }
    }
    let samples = to_i16(&planar);
    progress(decode_part + process_part);

    let bytes = match opts.format {
        Format::Wav => wav_pcm16(&samples, u16::from(channels), rate),
        Format::WavUlaw => wav_g711(&samples, 7, ulaw),
        Format::WavAlaw => wav_g711(&samples, 6, alaw),
        Format::Gsm => encode_gsm(&samples, false)?,
        Format::WavGsm => wav_gsm(&samples)?,
        Format::Mp3 => encode_mp3(&samples, channels, rate, opts.bitrate.unwrap_or(128))?,
    };

    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("audio");
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let path = free_path(out_dir, stem, suffix, opts.format.extension());
    let mut file = BufWriter::new(File::create(&path).map_err(|e| e.to_string())?);
    file.write_all(&bytes).and_then(|()| file.flush()).map_err(|e| e.to_string())?;
    progress(1.0);
    Ok(Converted { path: path.to_string_lossy().into_owned(), bytes: bytes.len() as u64, seconds })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    job: String,
    fraction: f32,
}

/// Converte `input` e grava ao lado do original (ou em `out_dir`). O progresso sai em `convert://progress`.
#[tauri::command]
pub async fn convert_audio(
    app: AppHandle,
    job: String,
    input: String,
    out_dir: Option<String>,
    suffix: String,
    options: Options,
) -> Result<Converted, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let input = PathBuf::from(input);
        let dir = out_dir.map(PathBuf::from).or_else(|| input.parent().map(Path::to_path_buf)).ok_or("Pasta de destino inválida")?;
        let mut last = -1.0f32;
        convert(&input, &dir, &suffix, &options, &mut |fraction| {
            // no máximo ~50 eventos por conversão
            if fraction - last >= 0.02 || fraction >= 1.0 {
                last = fraction;
                let _ = app.emit(PROGRESS_EVENT, Progress { job: job.clone(), fraction });
            }
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, seconds: f32) -> Vec<f32> {
        (0..(rate as f32 * seconds) as usize).map(|i| (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin() * 0.5).collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn resample_keeps_tone_and_length() {
        let out = resample(&sine(440.0, 44_100, 1.0), 44_100, 8000);
        assert!((out.len() as i64 - 8000).abs() <= 1);
        // longe das bordas o tom segue com a mesma energia (0,5/√2)
        assert!((rms(&out[400..out.len() - 400]) - 0.3535).abs() < 0.01, "rms {}", rms(&out));
    }

    #[test]
    fn resample_removes_content_above_new_nyquist() {
        let out = resample(&sine(10_000.0, 44_100, 1.0), 44_100, 8000);
        assert!(rms(&out[400..out.len() - 400]) < 0.005);
    }

    #[test]
    fn resample_up_keeps_energy() {
        let out = resample(&sine(1000.0, 8000, 1.0), 8000, 48_000);
        assert!((out.len() as i64 - 48_000).abs() <= 1);
        assert!((rms(&out[2000..out.len() - 2000]) - 0.3535).abs() < 0.01);
    }

    #[test]
    fn g711_matches_reference_points() {
        assert_eq!(ulaw(0), 0xFF);
        assert_eq!(ulaw(-1), 0x7F);
        assert_eq!(ulaw(32767), 0x80);
        assert_eq!(alaw(0), 0xD5);
        assert_eq!(alaw(-1), 0x55);
        assert_eq!(alaw(32767), 0xAA);
    }

    #[test]
    fn gsm_roundtrip_is_close() {
        let pcm = to_i16(&[sine(300.0, 8000, 1.0)]);
        for wav49 in [false, true] {
            let packed = encode_gsm(&pcm, wav49).unwrap();
            let back = decode_gsm(&packed, wav49).unwrap();
            assert_eq!(packed.len(), if wav49 { 25 * 65 } else { 50 * 33 });
            // o codec é com perdas e tem atraso: compara a energia, não amostra a amostra
            let a: Vec<f32> = pcm.iter().map(|&s| f32::from(s) / 32768.0).collect();
            let b: Vec<f32> = back.iter().map(|&s| f32::from(s) / 32768.0).collect();
            assert!((rms(&a[800..7000]) - rms(&b[800..7000])).abs() < 0.05);
        }
    }

    #[test]
    fn mp3_rates_snap_to_supported() {
        assert_eq!(mp3_rate(44_100), 44_100);
        assert_eq!(mp3_rate(96_000), 48_000);
        assert_eq!(mp3_rate(8_000), 8_000);
        assert_eq!(mp3_rate(10_000), 11_025);
    }

    #[test]
    fn mono_mix_and_duplicate() {
        let stereo = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        assert_eq!(set_channels(stereo, 1), vec![vec![0.5, 0.5]]);
        assert_eq!(set_channels(vec![vec![0.25]], 2), vec![vec![0.25], vec![0.25]]);
    }

    /// Gera um arquivo de cada formato em `$CONVERT_OUT` pra conferir com outro programa (`cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn write_samples() {
        let dir = PathBuf::from(std::env::var("CONVERT_OUT").expect("defina CONVERT_OUT"));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("tom.wav");
        let tone = [sine(440.0, 44_100, 2.0), sine(660.0, 44_100, 2.0)];
        fs::write(&input, wav_pcm16(&to_i16(&tone), 2, 44_100)).unwrap();
        for (name, format, rate, channels) in [
            ("wav8k", Format::Wav, Some(8000), Some(1)),
            ("wav44", Format::Wav, None, None),
            ("ulaw", Format::WavUlaw, None, None),
            ("alaw", Format::WavAlaw, None, None),
            ("gsm", Format::Gsm, None, None),
            ("wav49", Format::WavGsm, None, None),
            ("mp3", Format::Mp3, Some(44_100), Some(2)),
        ] {
            let opts = Options { format, sample_rate: rate, channels, bitrate: Some(128), normalize: false };
            let done = convert(&input, &dir, name, &opts, &mut |_| {}).unwrap();
            println!("{name}: {} ({} bytes)", done.path, done.bytes);
        }
    }

    #[test]
    fn mp3_mono_and_stereo() {
        let tone = to_i16(&[sine(440.0, 44_100, 1.0)]);
        let mono = encode_mp3(&tone, 1, 44_100, 128).unwrap();
        assert!(mono.len() > 1000, "mono: {} bytes", mono.len());
        let stereo_pcm: Vec<i16> = tone.iter().flat_map(|&s| [s, s]).collect();
        let stereo = encode_mp3(&stereo_pcm, 2, 44_100, 128).unwrap();
        assert!(stereo.len() > 1000, "estéreo: {} bytes", stereo.len());
    }

    /// Converte `$CONVERT_IN` pra WAV 16 bits na taxa original, em `$CONVERT_OUT` (`cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn convert_file_to_wav() {
        let input = PathBuf::from(std::env::var("CONVERT_IN").expect("defina CONVERT_IN"));
        let dir = PathBuf::from(std::env::var("CONVERT_OUT").expect("defina CONVERT_OUT"));
        let opts = Options { format: Format::Wav, sample_rate: None, channels: None, bitrate: None, normalize: false };
        let done = convert(&input, &dir, "decoded", &opts, &mut |_| {}).unwrap();
        println!("DECODED {}", done.path);
    }

    #[test]
    fn free_path_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("dl-convert-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let first = free_path(&dir, "a", "8k", "wav");
        fs::write(&first, b"x").unwrap();
        assert_eq!(free_path(&dir, "a", "8k", "wav"), dir.join("a_8k (2).wav"));
        fs::remove_dir_all(&dir).unwrap();
    }
}

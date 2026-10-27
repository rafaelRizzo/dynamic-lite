//! SFX com os sons do sistema (`C:\Windows\Media`), sem dependência de áudio: o WAV é lido, o volume aplicado
//! nas amostras e o resultado tocado da memória com `PlaySound`.

use std::fs;
use std::path::PathBuf;
use std::thread;

use windows::core::PCWSTR;
use windows::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_SYNC};

/// Som padrão quando o salvo não existe (as Settings vêm do macOS ou de outra máquina).
pub const DEFAULT: &str = "Windows Notify System Generic";

fn dir() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    PathBuf::from(root).join("Media")
}

/// Só nomes simples (letras, números, espaço, `-`, `_`, `.`): nada de caminhos.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !name.contains("..")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
}

/// Nomes (sem extensão) dos sons do sistema, ordenados.
#[tauri::command]
pub fn list_sounds() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|f| f.strip_suffix(".wav").map(String::from))
        .collect();
    names.sort();
    names
}

/// Aplica `volume` (0-1) num WAV PCM de 16 bits, no próprio buffer. Outros formatos tocam como estão.
fn apply_volume(wav: &mut [u8], volume: f64) {
    if wav.len() < 12 || &wav[0..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return;
    }
    let (mut pos, mut pcm16) = (12, false);
    while pos + 8 <= wav.len() {
        let id = [wav[pos], wav[pos + 1], wav[pos + 2], wav[pos + 3]];
        let size = u32::from_le_bytes([wav[pos + 4], wav[pos + 5], wav[pos + 6], wav[pos + 7]]) as usize;
        let body = pos + 8;
        let end = body.saturating_add(size).min(wav.len());
        match &id {
            b"fmt " if end - body >= 16 => {
                let tag = u16::from_le_bytes([wav[body], wav[body + 1]]);
                let bits = u16::from_le_bytes([wav[body + 14], wav[body + 15]]);
                pcm16 = tag == 1 && bits == 16;
            }
            b"data" => {
                if pcm16 {
                    for s in wav[body..end].chunks_exact_mut(2) {
                        let v = f64::from(i16::from_le_bytes([s[0], s[1]])) * volume;
                        s.copy_from_slice(&(v.round() as i16).to_le_bytes());
                    }
                }
                return;
            }
            _ => {}
        }
        // chunks RIFF têm tamanho par
        pos = body.saturating_add(size).saturating_add(size & 1);
    }
}

/// Toca `name` com volume 0-1. Não bloqueia.
pub fn play(name: &str, volume: f64) {
    if !valid_name(name) {
        return;
    }
    let mut path = dir().join(format!("{name}.wav"));
    if !path.is_file() {
        path = dir().join(format!("{DEFAULT}.wav"));
    }
    thread::spawn(move || {
        let Ok(mut wav) = fs::read(path) else { return };
        apply_volume(&mut wav, volume.clamp(0.0, 1.0));
        // SAFETY: com SND_MEMORY o ponteiro é o WAV inteiro, e SND_SYNC só volta depois de tocar: o buffer vive até lá
        unsafe {
            let _ = PlaySoundW(PCWSTR(wav.as_ptr().cast()), None, SND_MEMORY | SND_SYNC | SND_NODEFAULT);
        }
    });
}

#[tauri::command]
pub fn play_sound(name: String, volume: f64) {
    play(&name, volume);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend([1, 0, 1, 0]);
        out.extend(8000u32.to_le_bytes());
        out.extend(16000u32.to_le_bytes());
        out.extend([2, 0, 16, 0]);
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    #[test]
    fn volume_scales_pcm16() {
        let mut w = wav(&[1000, -2000]);
        apply_volume(&mut w, 0.5);
        let n = w.len();
        assert_eq!(i16::from_le_bytes([w[n - 4], w[n - 3]]), 500);
        assert_eq!(i16::from_le_bytes([w[n - 2], w[n - 1]]), -1000);
    }

    #[test]
    fn names_cannot_escape_the_folder() {
        assert!(valid_name("Windows Notify System Generic"));
        assert!(!valid_name(r"..\evil"));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(""));
    }
}

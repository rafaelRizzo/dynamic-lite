//! SFX com os sons do sistema (`afplay`), sem dependência de áudio.

use std::fs;
use std::process::Command;
use std::thread;

const DIR: &str = "/System/Library/Sounds";

/// Nomes (sem extensão) dos sons do sistema, ordenados.
#[tauri::command]
pub fn list_sounds() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(DIR)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|f| f.strip_suffix(".aiff").map(String::from))
        .collect();
    names.sort();
    names
}

/// Toca `name` (só nomes simples: nada de caminhos) com volume 0-1. Não bloqueia.
pub fn play(name: &str, volume: f64) {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return;
    }
    let path = format!("{DIR}/{name}.aiff");
    let Ok(mut child) = Command::new("afplay").arg("-v").arg(format!("{:.2}", volume.clamp(0.0, 1.0))).arg(path).spawn() else {
        return;
    };
    // recolhe o processo quando acabar, sem deixar zumbi
    thread::spawn(move || {
        let _ = child.wait();
    });
}

#[tauri::command]
pub fn play_sound(name: String, volume: f64) {
    play(&name, volume);
}

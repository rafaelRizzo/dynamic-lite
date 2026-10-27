//! Volume do Spotify no mixer do Windows (sessão de áudio do app). O Spotify não expõe o volume interno por
//! fora, então o controle da Island mexe no mesmo slider do "Mixer de volume" do sistema.

use windows::core::Interface;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume,
    MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

use crate::win;

/// Volumes de todas as sessões de áudio do Spotify (ele abre uma por processo que toca).
fn spotify_sessions() -> Vec<ISimpleAudioVolume> {
    win::com_init();
    let mut found = vec![];
    // SAFETY: chamadas COM com objetos recém-criados; o identificador devolvido é liberado logo após o uso
    let _: Option<()> = (|| unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).ok()?;
        let sessions = manager.GetSessionEnumerator().ok()?;
        for i in 0..sessions.GetCount().ok()? {
            let Ok(control) = sessions.GetSession(i) else { continue };
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else { continue };
            let Ok(id) = control2.GetSessionIdentifier() else { continue };
            let is_spotify = id.to_string().unwrap_or_default().to_ascii_lowercase().contains("spotify.exe");
            CoTaskMemFree(Some(id.0.cast()));
            if is_spotify {
                if let Ok(volume) = control.cast::<ISimpleAudioVolume>() {
                    found.push(volume);
                }
            }
        }
        Some(())
    })();
    found
}

/// Volume do Spotify, 0-100. `None` se ele não tem sessão de áudio (fechado ou nunca tocou).
pub fn get() -> Option<u8> {
    let volume = spotify_sessions().into_iter().next()?;
    let level = unsafe { volume.GetMasterVolume() }.ok()?;
    Some((level * 100.0).round().clamp(0.0, 100.0) as u8)
}

pub fn set(percent: u8) {
    let level = f32::from(percent.min(100)) / 100.0;
    for volume in spotify_sessions() {
        let _ = unsafe { volume.SetMasterVolume(level, std::ptr::null()) };
    }
}

//! Bateria dos dispositivos Bluetooth pareados (fones, mouse, teclado...). O Windows grava o nível em
//! `DEVPKEY_Device_BatteryLevel` num nó PnP do aparelho (o do serviço Handsfree nos fones clássicos, o próprio nó
//! nos LE), não no dispositivo pareado: por isso os dois são cruzados pelo endereço Bluetooth.

use std::collections::HashMap;

use serde::Serialize;
use windows::core::{Interface, HSTRING};
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::Pnp::{PnpObject, PnpObjectType};
use windows::Devices::Enumeration::DeviceInformation;
use windows::Foundation::IReference;
use windows_collections::IIterable;

use crate::win;

const BATTERY_KEY: &str = "{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2";
/// `DEVPKEY_Device_IsPresent`: nós antigos de um fone já pareado em outro adaptador continuam com o último nível.
const PRESENT_KEY: &str = "{540B947E-8B40-45BC-A8A2-6A0B894CBDA2} 5";
const CONNECTED_KEY: &str = "System.Devices.Aep.IsConnected";

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothBattery {
    pub id: String,
    pub name: String,
    /// 0-100; `None` se o dispositivo não informa bateria
    pub battery: Option<u8>,
    pub connected: bool,
}

fn lookup<T: windows::core::RuntimeType + 'static>(
    map: &windows_collections::IMapView<HSTRING, windows::core::IInspectable>,
    key: &str,
) -> Option<T>
where
    IReference<T>: Interface,
{
    map.Lookup(&HSTRING::from(key)).ok()?.cast::<IReference<T>>().ok()?.Value().ok()
}

fn is_hex(s: &str) -> bool {
    s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Endereço (12 hex, minúsculo) de um id de dispositivo pareado: `...-d4:12:f1:54:e6:32`.
fn mac_of_paired(id: &str) -> Option<String> {
    let mac = id.rsplit('-').next()?.replace(':', "").to_ascii_lowercase();
    is_hex(&mac).then_some(mac)
}

/// Endereço de um id PnP: `BTHENUM\{...}\8&172c578d&0&D412F154E632_C00000000` ou `BTHLE\Dev_d0622ccb5b6f\...`.
fn mac_of_pnp(id: &str) -> Option<String> {
    let lower = id.to_ascii_lowercase();
    let mac = match lower.find("dev_") {
        Some(i) => lower.get(i + 4..i + 16)?.to_owned(),
        None => lower.rsplit('&').next()?.split('_').next()?.to_owned(),
    };
    is_hex(&mac).then_some(mac)
}

/// Nível de bateria por endereço (nós presentes); só vale para aparelho conectado.
fn batteries() -> HashMap<String, u8> {
    let props = IIterable::<HSTRING>::from(vec![HSTRING::from(BATTERY_KEY), HSTRING::from(PRESENT_KEY)]);
    let filter = HSTRING::from("System.Devices.DeviceInstanceId:~~\"BTH\"");
    let Ok(op) = PnpObject::FindAllAsyncAqsFilter(PnpObjectType::Device, &props, &filter) else { return HashMap::new() };
    let Ok(nodes) = op.join() else { return HashMap::new() };
    let mut out = HashMap::new();
    for node in nodes {
        let (Ok(id), Ok(map)) = (node.Id(), node.Properties()) else { continue };
        let Some(level) = lookup::<u8>(&map, BATTERY_KEY).filter(|b| *b <= 100) else { continue };
        if lookup::<bool>(&map, PRESENT_KEY) != Some(true) {
            continue;
        }
        if let Some(mac) = mac_of_pnp(&id.to_string()) {
            out.insert(mac, level);
        }
    }
    out
}

/// Estado real da conexão. `System.Devices.Aep.IsConnected` fica verdadeiro para aparelho só pareado e ausente.
fn is_connected(id: &HSTRING, le: bool) -> Option<bool> {
    let status = if le {
        BluetoothLEDevice::FromIdAsync(id).ok()?.join().ok()?.ConnectionStatus().ok()?
    } else {
        BluetoothDevice::FromIdAsync(id).ok()?.join().ok()?.ConnectionStatus().ok()?
    };
    Some(status == BluetoothConnectionStatus::Connected)
}

/// Dispositivos pareados (clássicos e LE): `(endereço, nome, conectado)`.
fn paired() -> Vec<(String, String, bool)> {
    let props = IIterable::<HSTRING>::from(vec![HSTRING::from(CONNECTED_KEY)]);
    let mut out = Vec::new();
    for (le, selector) in [
        (false, BluetoothDevice::GetDeviceSelectorFromPairingState(true)),
        (true, BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)),
    ] {
        let Ok(selector) = selector else { continue };
        let Ok(op) = DeviceInformation::FindAllAsyncAqsFilterAndAdditionalProperties(&selector, &props) else { continue };
        let Ok(devices) = op.join() else { continue };
        for info in devices {
            let (Ok(id), Ok(name), Ok(map)) = (info.Id(), info.Name(), info.Properties()) else { continue };
            let name = name.to_string();
            let Some(mac) = mac_of_paired(&id.to_string()) else { continue };
            if !name.trim().is_empty() {
                let connected = is_connected(&id, le).unwrap_or_else(|| lookup::<bool>(&map, CONNECTED_KEY).unwrap_or(false));
                out.push((mac, name, connected));
            }
        }
    }
    out
}

/// Dispositivos Bluetooth pareados. Conectados primeiro, depois por nome.
pub fn read() -> Vec<BluetoothBattery> {
    win::com_init();
    let levels = batteries();
    let mut all: Vec<BluetoothBattery> = Vec::new();
    for (mac, name, connected) in paired() {
        // o mesmo aparelho vem pelos dois protocolos: conectado se qualquer um disser que sim
        match all.iter_mut().find(|d| d.id == mac) {
            Some(existing) => existing.connected |= connected,
            None => all.push(BluetoothBattery { id: mac, name, battery: None, connected }),
        }
    }
    // nó PnP guarda o último nível mesmo com o aparelho desligado: só conta se está conectado
    for d in all.iter_mut().filter(|d| d.connected) {
        d.battery = levels.get(&d.id).copied();
    }
    all.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    all
}

#[tauri::command]
pub async fn bluetooth_batteries() -> Vec<BluetoothBattery> {
    tauri::async_runtime::spawn_blocking(read).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macs_from_ids() {
        assert_eq!(mac_of_paired("Bluetooth#Bluetooth00:1a:7d:da:71:12-d4:12:f1:54:e6:32").as_deref(), Some("d412f154e632"));
        assert_eq!(
            mac_of_pnp(r"BTHENUM\{0000111e-0000-1000-8000-00805f9b34fb}_VID&000105d6_PID&000a\8&172c578d&0&D412F154E632_C00000000").as_deref(),
            Some("d412f154e632")
        );
        assert_eq!(mac_of_pnp(r"BTHLE\Dev_d0622ccb5b6f\8&770e25&0&d0622ccb5b6f").as_deref(), Some("d0622ccb5b6f"));
    }

    #[test]
    fn lists_devices() {
        for d in read() {
            println!("{d:?}");
        }
    }
}

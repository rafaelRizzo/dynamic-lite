//! Bateria dos dispositivos Bluetooth pareados (fones, mouse, teclado...). O Windows guarda o nível em
//! `DEVPKEY_Device_BatteryLevel` do nó do dispositivo, que o WinRT devolve como propriedade extra da enumeração.

use serde::Serialize;
use windows::core::{Interface, HSTRING};
use windows::Devices::Bluetooth::{BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationKind};
use windows::Foundation::IReference;
use windows_collections::IIterable;

use crate::win;

const BATTERY_KEY: &str = "{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2";
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


fn query(selector: &HSTRING) -> Vec<BluetoothBattery> {
    let props = IIterable::<HSTRING>::from(vec![HSTRING::from(BATTERY_KEY), HSTRING::from(CONNECTED_KEY)]);
    let Ok(op) = DeviceInformation::FindAllAsyncAqsFilterAndAdditionalProperties(selector, &props) else {
        return vec![];
    };
    let Ok(devices) = op.join() else { return vec![] };
    let _ = DeviceInformationKind::Device;
    devices
        .into_iter()
        .filter_map(|info| {
            let name = info.Name().ok()?.to_string();
            if name.trim().is_empty() {
                return None;
            }
            let map = info.Properties().ok()?;
            let battery = map
                .Lookup(&HSTRING::from(BATTERY_KEY))
                .ok()
                .and_then(|v| v.cast::<IReference<u8>>().ok())
                .and_then(|r| r.Value().ok())
                .filter(|b| *b <= 100);
            let connected = map
                .Lookup(&HSTRING::from(CONNECTED_KEY))
                .ok()
                .and_then(|v| v.cast::<IReference<bool>>().ok())
                .and_then(|r| r.Value().ok())
                .unwrap_or(false);
            Some(BluetoothBattery { id: info.Id().ok()?.to_string(), name, battery, connected })
        })
        .collect()
}

/// Dispositivos Bluetooth (clássicos e LE) pareados. Conectados primeiro, depois por nome.
pub fn read() -> Vec<BluetoothBattery> {
    win::com_init();
    let mut all = Vec::new();
    for selector in [
        BluetoothDevice::GetDeviceSelectorFromPairingState(true),
        BluetoothLEDevice::GetDeviceSelectorFromPairingState(true),
    ] {
        let Ok(selector) = selector else { continue };
        for device in query(&selector) {
            // o mesmo aparelho pode vir pelos dois protocolos: fica o que tem bateria
            match all.iter_mut().find(|d: &&mut BluetoothBattery| d.name == device.name) {
                Some(existing) => {
                    if existing.battery.is_none() {
                        existing.battery = device.battery;
                    }
                    existing.connected |= device.connected;
                }
                None => all.push(device),
            }
        }
    }
    all.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    all
}

#[tauri::command]
pub async fn bluetooth_batteries() -> Vec<BluetoothBattery> {
    tauri::async_runtime::spawn_blocking(read).await.unwrap_or_default()
}

//! Bateria dos dispositivos Bluetooth pareados (fones, mouse, teclado...). O `system_profiler` já junta o que o
//! macOS sabe de cada aparelho, inclusive o nível de bateria que o dispositivo informa.

use std::process::Command;

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothBattery {
    pub id: String,
    pub name: String,
    /// 0-100; `None` se o dispositivo não informa bateria
    pub battery: Option<u8>,
    pub connected: bool,
}

/// "85%" -> 85
fn percent(v: Option<&Value>) -> Option<u8> {
    let n: u8 = v?.as_str()?.trim().trim_end_matches('%').parse().ok()?;
    (n <= 100).then_some(n)
}

/// Fones com dois lados informam esquerdo e direito: vale o pior, que é o que acaba primeiro.
fn battery_of(props: &Value) -> Option<u8> {
    let main = percent(props.get("device_batteryLevelMain"));
    let sides = [percent(props.get("device_batteryLevelLeft")), percent(props.get("device_batteryLevelRight"))];
    main.or_else(|| sides.into_iter().flatten().min())
}

fn collect(list: Option<&Value>, connected: bool, out: &mut Vec<BluetoothBattery>) {
    // cada item é um objeto de uma chave só: { "Nome do aparelho": { ...propriedades } }
    for item in list.and_then(Value::as_array).into_iter().flatten() {
        for (name, props) in item.as_object().into_iter().flatten() {
            let id = props.get("device_address").and_then(Value::as_str).unwrap_or(name).to_owned();
            out.push(BluetoothBattery { id, name: name.clone(), battery: battery_of(props), connected });
        }
    }
}

/// Conectados primeiro, depois por nome.
fn parse(json: &str) -> Vec<BluetoothBattery> {
    let Ok(root) = serde_json::from_str::<Value>(json) else { return vec![] };
    let mut out = Vec::new();
    for controller in root.get("SPBluetoothDataType").and_then(Value::as_array).into_iter().flatten() {
        collect(controller.get("device_connected"), true, &mut out);
        collect(controller.get("device_not_connected"), false, &mut out);
    }
    out.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

pub fn read() -> Vec<BluetoothBattery> {
    Command::new("/usr/sbin/system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

#[tauri::command]
pub async fn bluetooth_batteries() -> Vec<BluetoothBattery> {
    tauri::async_runtime::spawn_blocking(read).await.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"SPBluetoothDataType":[{
        "device_connected":[
            {"AirPods":{"device_address":"AA:01","device_batteryLevelLeft":"15%","device_batteryLevelRight":"80%","device_batteryLevelCase":"90%"}},
            {"Magic Mouse":{"device_address":"AA:02","device_batteryLevelMain":"62%"}}
        ],
        "device_not_connected":[{"Alto-falante":{"device_address":"AA:03"}}]
    }]}"#;

    #[test]
    fn reads_levels_and_sorts_connected_first() {
        let d = parse(SAMPLE);
        let view: Vec<_> = d.iter().map(|x| (x.name.as_str(), x.battery, x.connected)).collect();
        assert_eq!(view, [("AirPods", Some(15), true), ("Magic Mouse", Some(62), true), ("Alto-falante", None, false)]);
    }

    #[test]
    fn bad_json_is_empty() {
        assert!(parse("não é json").is_empty());
    }
}

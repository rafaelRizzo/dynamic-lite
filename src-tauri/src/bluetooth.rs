//! Bateria dos dispositivos Bluetooth pareados (fones, mouse, teclado...). O `system_profiler` já junta o que o
//! macOS sabe de cada aparelho, inclusive o nível de bateria que o dispositivo informa.

use std::collections::HashMap;
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

/// Fones HFP/A2DP (ex.: genéricos) não aparecem com bateria no `system_profiler`, mas o macOS os lista em
/// `pmset -g accps`: " -AZ09 (id=123)\t90%; discharging present: true". Mapa nome -> nível.
fn parse_accps(text: &str) -> HashMap<String, u8> {
    text.lines()
        .filter_map(|line| {
            let (head, rest) = line.trim().strip_prefix('-')?.split_once(" (id=")?;
            let pct = rest.split_once('\t')?.1.split('%').next()?.trim().parse::<u8>().ok()?;
            (pct <= 100).then(|| (head.to_owned(), pct))
        })
        .collect()
}

/// Mouse/teclado/trackpad Apple não saem nem no `system_profiler` nem no `pmset` (nome vazio): a bateria está
/// no IORegistry, com `DeviceAddress = "bc-d0-74-dd-ae-a2"`. Mapa endereço (só hex, minúsculo) -> nível.
fn parse_ioreg(text: &str) -> HashMap<String, u8> {
    fn flush(addr: &mut Option<String>, pct: &mut Option<u8>, out: &mut HashMap<String, u8>) {
        if let (Some(a), Some(p)) = (addr.take(), pct.take()) {
            out.insert(a, p);
        }
        *addr = None;
        *pct = None;
    }
    let (mut out, mut addr, mut pct) = (HashMap::new(), None, None);
    for line in text.lines() {
        if line.contains("+-o ") {
            flush(&mut addr, &mut pct, &mut out);
        } else if let Some(v) = line.split_once("\"BatteryPercent\" = ") {
            pct = v.1.trim().parse::<u8>().ok().filter(|n| *n <= 100);
        } else if let Some(v) = line.split_once("\"DeviceAddress\" = ") {
            addr = Some(addr_key(v.1.trim().trim_matches('"')));
        }
    }
    flush(&mut addr, &mut pct, &mut out);
    out
}

/// "BC:D0:74:DD:AE:A2" / "bc-d0-74-dd-ae-a2" -> "bcd074ddaea2"
fn addr_key(a: &str) -> String {
    a.chars().filter(char::is_ascii_hexdigit).collect::<String>().to_lowercase()
}

fn run(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok().filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn read() -> Vec<BluetoothBattery> {
    let mut devices = run("/usr/sbin/system_profiler", &["SPBluetoothDataType", "-json"]).map(|j| parse(&j)).unwrap_or_default();
    if devices.iter().any(|d| d.connected && d.battery.is_none()) {
        let accps = run("/usr/bin/pmset", &["-g", "accps"]).map(|t| parse_accps(&t)).unwrap_or_default();
        let hid = run("/usr/sbin/ioreg", &["-r", "-k", "BatteryPercent", "-l"]).map(|t| parse_ioreg(&t)).unwrap_or_default();
        for d in devices.iter_mut().filter(|d| d.connected && d.battery.is_none()) {
            d.battery = hid.get(&addr_key(&d.id)).or_else(|| accps.get(&d.name)).copied();
        }
    }
    devices
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
    fn reads_accps_levels() {
        let m = parse_accps("Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t100%; charged; 0:00 remaining present: true\n -AZ09 (id=2)\t90%; discharging present: true\n");
        assert_eq!(m.get("AZ09"), Some(&90));
        assert_eq!(m.get("InternalBattery-0"), Some(&100));
    }

    #[test]
    fn reads_ioreg_levels() {
        let t = "+-o AppleDeviceManagementHIDEventService  <class X>\n  |   \"Product\" = \"\"\n  |   \"BatteryPercent\" = 16\n  |   \"DeviceAddress\" = \"bc-d0-74-dd-ae-a2\"\n  +-o IOHIDEventServiceUserClient  <class Y>\n";
        assert_eq!(parse_ioreg(t).get(&addr_key("BC:D0:74:DD:AE:A2")), Some(&16));
    }

    #[test]
    fn bad_json_is_empty() {
        assert!(parse("não é json").is_empty());
    }
}

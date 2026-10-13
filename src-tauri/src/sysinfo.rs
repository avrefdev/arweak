use crate::{memory, util};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Row {
    pub name: &'static str,
    pub value: String,
    pub tone: &'static str,
}

#[derive(Serialize, Clone, Default)]
pub struct System {
    pub hardware: Vec<Row>,
    pub memory: Vec<Row>,
    pub display: Vec<Row>,
    pub dual_channel: bool,
    pub expo: bool,
    pub warning: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Raw {
    cpu: String,
    gpu: String,
    driver: String,
    refresh: u32,
    sticks: Vec<Stick>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
struct Stick {
    capacity: u64,
    speed: u32,
    configured_clock_speed: u32,
}

const SCRIPT: &str = r#"
$c = (Get-CimInstance Win32_Processor | Select-Object -First 1).Name.Trim()
$v = Get-CimInstance Win32_VideoController | Where-Object { $_.Name -notmatch 'Microsoft|Virtual|Parsec|Remote' } | Sort-Object AdapterRAM -Descending | Select-Object -First 1
$m = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Capacity, Speed, ConfiguredClockSpeed)
[pscustomobject]@{ Cpu = $c; Gpu = $v.Name; Driver = $v.DriverVersion; Refresh = [int]$v.CurrentRefreshRate; Sticks = $m } | ConvertTo-Json -Depth 3 -Compress
"#;

fn nvidia_version(raw: &str) -> String {
    let digits: String = raw.split('.').rev().take(2).collect::<Vec<_>>().into_iter().rev().collect::<String>();
    if digits.len() >= 5 {
        let d = &digits[digits.len() - 5..];
        format!("{}.{}", &d[..3], &d[3..])
    } else {
        raw.to_string()
    }
}

fn power_plan() -> String {
    let out = util::stdout("powercfg.exe", "/getactivescheme");
    out.rsplit_once('(')
        .and_then(|(_, r)| r.split_once(')'))
        .map(|(n, _)| n.trim().to_string())
        .unwrap_or_else(|| "—".into())
}

fn reg_on(key: &str, name: &str, on: &str) -> Option<bool> {
    util::reg_get(key, name).map(|(_, v)| v == on)
}

pub fn read() -> System {
    let raw: Raw = serde_json::from_str(&util::ps(SCRIPT)).unwrap_or_default();
    let total_gb = (raw.sticks.iter().map(|s| s.capacity).sum::<u64>() as f64 / 1073741824.0).round() as u64;
    let rated = raw.sticks.iter().map(|s| s.speed).max().unwrap_or(0);
    let actual = raw.sticks.iter().map(|s| s.configured_clock_speed).max().unwrap_or(0);
    let dual = raw.sticks.len() >= 2;
    let expo = rated == 0 || actual >= rated;
    let free = memory::status().avail as f64 / 1073741824.0;
    let driver = if raw.gpu.to_lowercase().contains("nvidia") { nvidia_version(&raw.driver) } else { raw.driver.clone() };
    let plan = power_plan();
    let plan_ok = ["ultimate", "nihai", "yüksek", "high"].iter().any(|k| plan.to_lowercase().contains(k));
    let gamemode = reg_on(r"HKCU\Software\Microsoft\GameBar", "AutoGameModeEnabled", "0x1").unwrap_or(true);
    let hags = reg_on(r"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", "0x2").unwrap_or(false);

    let row = |name, value: String, tone| Row { name, value, tone };
    System {
        hardware: vec![
            row("İşlemci", raw.cpu.replace("6-Core Processor", "").replace("8-Core Processor", "").trim().to_string(), "plain"),
            row("Ekran kartı", raw.gpu.replace("NVIDIA GeForce ", "").replace("AMD Radeon ", "Radeon "), "plain"),
            row("RAM", format!("{total_gb} GB"), "plain"),
            row("Sürücü", driver, "plain"),
        ],
        memory: vec![
            row("Kanal", if dual { "Çift kanal".into() } else { "Tek kanal".into() }, if dual { "ok" } else { "err" }),
            row("Hız", if rated > 0 { format!("{actual} / {rated}") } else { "—".into() }, if expo { "ok" } else { "warn" }),
            row("EXPO / XMP", if expo { "Açık".into() } else { "Kapalı".into() }, if expo { "ok" } else { "warn" }),
            row("Boş bellek", format!("{free:.1} GB").replace('.', ","), "plain"),
        ],
        display: vec![
            row("Yenileme hızı", format!("{} Hz", raw.refresh), if raw.refresh >= 144 { "ok" } else { "warn" }),
            row("Oyun Modu", if gamemode { "Açık".into() } else { "Kapalı".into() }, if gamemode { "ok" } else { "warn" }),
            row("HAGS", if hags { "Açık".into() } else { "Kapalı".into() }, if hags { "ok" } else { "warn" }),
            row("Güç planı", plan, if plan_ok { "ok" } else { "warn" }),
        ],
        dual_channel: dual,
        expo,
        warning: if !dual {
            Some("İkinci bir RAM ile çift kanala geçin".into())
        } else if !expo {
            Some("BIOS'ta EXPO / XMP profilini açın".into())
        } else {
            None
        },
    }
}

pub fn score(sys: &System, applied: usize, total: usize) -> u32 {
    let mut s = 40.0;
    if sys.dual_channel {
        s += 15.0;
    }
    if sys.expo {
        s += 10.0;
    }
    s += 35.0 * applied as f64 / total.max(1) as f64;
    s.round().min(100.0) as u32
}

use crate::{games, util};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct Tweak {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub recommended: bool,
    pub runtime: bool,
    pub reboot: bool,
}

pub const TWEAKS: &[Tweak] = &[
    Tweak { id: "power", name: "Ultimate güç planı", group: "Sistem", recommended: true, runtime: false, reboot: false },
    Tweak { id: "gamebar", name: "Game Bar kaydını kapat", group: "Sistem", recommended: true, runtime: false, reboot: false },
    Tweak { id: "prio", name: "Oyuna yüksek öncelik", group: "Sistem", recommended: true, runtime: true, reboot: false },
    Tweak { id: "bg", name: "Oyunda arka planı kıs", group: "Sistem", recommended: false, runtime: true, reboot: false },
    Tweak { id: "gpu", name: "GPU maksimum performans", group: "Ekran kartı", recommended: true, runtime: false, reboot: false },
    Tweak { id: "gamemode", name: "Windows Oyun Modu", group: "Ekran kartı", recommended: true, runtime: false, reboot: false },
    Tweak { id: "hags", name: "Donanım hızlandırmalı zamanlama", group: "Ekran kartı", recommended: true, runtime: false, reboot: true },
    Tweak { id: "mpo", name: "Çoklu düzlem kaplamasını kapat", group: "Ekran kartı", recommended: false, runtime: false, reboot: true },
    Tweak { id: "ram", name: "Akıllı RAM temizleme", group: "Bellek ve ağ", recommended: true, runtime: true, reboot: false },
    Tweak { id: "net", name: "Ağ güç tasarrufunu kapat", group: "Bellek ve ağ", recommended: true, runtime: false, reboot: false },
    Tweak { id: "nagle", name: "Nagle algoritmasını kapat", group: "Bellek ve ağ", recommended: false, runtime: false, reboot: false },
    Tweak { id: "timer", name: "Yüksek çözünürlüklü zamanlayıcı", group: "Bellek ve ağ", recommended: true, runtime: false, reboot: true },
];

pub fn recommended() -> HashMap<String, bool> {
    TWEAKS.iter().map(|t| (t.id.to_string(), t.recommended)).collect()
}

pub fn get(id: &str) -> Option<&'static Tweak> {
    TWEAKS.iter().find(|t| t.id == id)
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Undo {
    Reg { key: String, name: String, prev: Option<(String, String)> },
    PowerScheme(String),
    PowerValue { sub: String, setting: String, ac: String, dc: String },
    Ps(String),
}

fn backup_path() -> std::path::PathBuf {
    util::data_dir().join("backup.json")
}

pub fn load_backup() -> HashMap<String, Vec<Undo>> {
    std::fs::read_to_string(backup_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_backup(b: &HashMap<String, Vec<Undo>>) {
    if let Ok(json) = serde_json::to_string_pretty(b) {
        let _ = std::fs::write(backup_path(), json);
    }
}

fn set_reg(undo: &mut Vec<Undo>, key: &str, name: &str, kind: &str, data: &str) {
    let prev = util::reg_get(key, name);
    if util::reg_set(key, name, kind, data) {
        undo.push(Undo::Reg { key: key.into(), name: name.into(), prev });
    }
}

fn active_scheme() -> Option<String> {
    let out = util::stdout("powercfg.exe", "/getactivescheme");
    out.split_whitespace()
        .find(|w| w.len() == 36 && w.matches('-').count() == 4)
        .map(String::from)
}

fn power_value(sub: &str, setting: &str) -> Option<(String, String)> {
    let out = util::stdout("powercfg.exe", &format!("/query SCHEME_CURRENT {sub} {setting}"));
    let vals: Vec<String> = out
        .lines()
        .filter_map(|l| l.trim().rsplit_once(": 0x").map(|(_, v)| format!("0x{}", v.trim())))
        .collect();
    (vals.len() >= 2).then(|| (vals[vals.len() - 2].clone(), vals[vals.len() - 1].clone()))
}

fn set_power_value(undo: &mut Vec<Undo>, sub: &str, setting: &str, value: u32) {
    if let Some((ac, dc)) = power_value(sub, setting) {
        undo.push(Undo::PowerValue { sub: sub.into(), setting: setting.into(), ac, dc });
    }
    let _ = util::run("powercfg.exe", &format!("/setacvalueindex SCHEME_CURRENT {sub} {setting} {value}"));
    let _ = util::run("powercfg.exe", &format!("/setdcvalueindex SCHEME_CURRENT {sub} {setting} {value}"));
    let _ = util::run("powercfg.exe", "/setactive SCHEME_CURRENT");
}

fn ultimate_scheme(known: &str) -> Option<String> {
    let list = util::stdout("powercfg.exe", "/list");
    if !known.is_empty() && list.contains(known) {
        return Some(known.to_string());
    }
    let out = util::stdout("powercfg.exe", "-duplicatescheme e9a42b02-d5df-448d-aa00-03f14749eb61");
    out.split_whitespace()
        .find(|w| w.len() == 36 && w.matches('-').count() == 4)
        .map(String::from)
}

const DIRECTX_PREF: &str = r"HKCU\Software\Microsoft\DirectX\UserGpuPreferences";
const TCPIP_IF: &str = r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

pub fn apply(id: &str, ultimate: &mut String) -> Vec<Undo> {
    let mut undo = Vec::new();
    match id {
        "power" => {
            if let (Some(prev), Some(guid)) = (active_scheme(), ultimate_scheme(ultimate)) {
                if prev != guid {
                    undo.push(Undo::PowerScheme(prev));
                    let _ = util::run("powercfg.exe", &format!("/setactive {guid}"));
                }
                *ultimate = guid;
            }
        }
        "gamebar" => {
            set_reg(&mut undo, r"HKCU\System\GameConfigStore", "GameDVR_Enabled", "REG_DWORD", "0");
            set_reg(&mut undo, r"HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", "REG_DWORD", "0");
        }
        "gamemode" => {
            set_reg(&mut undo, r"HKCU\Software\Microsoft\GameBar", "AutoGameModeEnabled", "REG_DWORD", "1");
            set_reg(&mut undo, r"HKCU\Software\Microsoft\GameBar", "AllowAutoGameMode", "REG_DWORD", "1");
        }
        "gpu" => {
            set_power_value(&mut undo, "SUB_PCIEXPRESS", "ASPM", 0);
            for g in games::GAMES {
                if let Some(p) = games::exe_path(g).filter(|p| p.exists()) {
                    set_reg(&mut undo, DIRECTX_PREF, &p.to_string_lossy(), "REG_SZ", "GpuPreference=2;");
                }
            }
        }
        "hags" => set_reg(&mut undo, r"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers", "HwSchMode", "REG_DWORD", "2"),
        "mpo" => set_reg(&mut undo, r"HKLM\SOFTWARE\Microsoft\Windows\Dwm", "OverlayTestMode", "REG_DWORD", "5"),
        "timer" => set_reg(&mut undo, r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\kernel", "GlobalTimerResolutionRequests", "REG_DWORD", "1"),
        "nagle" => {
            for key in util::reg_subkeys(TCPIP_IF) {
                let has_ip = util::reg_get(&key, "DhcpIPAddress").is_some_and(|(_, v)| v != "0.0.0.0" && !v.is_empty())
                    || util::reg_get(&key, "IPAddress").is_some_and(|(_, v)| !v.is_empty() && v != "0.0.0.0");
                if has_ip {
                    set_reg(&mut undo, &key, "TcpAckFrequency", "REG_DWORD", "1");
                    set_reg(&mut undo, &key, "TCPNoDelay", "REG_DWORD", "1");
                }
            }
        }
        "net" => {
            let names = util::ps(
                "Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | ForEach-Object { $p = Get-NetAdapterPowerManagement -Name $_.Name -ErrorAction SilentlyContinue; if ($p -and $p.AllowComputerToTurnOffDevice -eq 'Enabled') { $_.Name } }",
            );
            let names: Vec<&str> = names.lines().map(str::trim).filter(|s| !s.is_empty()).collect();
            if !names.is_empty() {
                let list = names.iter().map(|n| format!("'{}'", n.replace('\'', "''"))).collect::<Vec<_>>().join(",");
                util::ps(&format!("Disable-NetAdapterPowerManagement -Name {list} -NoRestart -ErrorAction SilentlyContinue"));
                undo.push(Undo::Ps(format!("Enable-NetAdapterPowerManagement -Name {list} -NoRestart -ErrorAction SilentlyContinue")));
            }
        }
        _ => {}
    }
    undo
}

pub fn revert(undo: &[Undo]) {
    for u in undo.iter().rev() {
        match u {
            Undo::Reg { key, name, prev: Some((kind, data)) } => {
                util::reg_set(key, name, kind, data);
            }
            Undo::Reg { key, name, prev: None } => util::reg_del(key, name),
            Undo::PowerScheme(guid) => {
                let _ = util::run("powercfg.exe", &format!("/setactive {guid}"));
            }
            Undo::PowerValue { sub, setting, ac, dc } => {
                let _ = util::run("powercfg.exe", &format!("/setacvalueindex SCHEME_CURRENT {sub} {setting} {ac}"));
                let _ = util::run("powercfg.exe", &format!("/setdcvalueindex SCHEME_CURRENT {sub} {setting} {dc}"));
                let _ = util::run("powercfg.exe", "/setactive SCHEME_CURRENT");
            }
            Undo::Ps(script) => {
                util::ps(script);
            }
        }
    }
}

pub fn apply_selected(selected: &HashMap<String, bool>, ultimate: &mut String) -> bool {
    let mut backup = load_backup();
    let mut reboot = false;
    for t in TWEAKS.iter().filter(|t| !t.runtime) {
        let on = selected.get(t.id).copied().unwrap_or(false);
        if on && !backup.contains_key(t.id) {
            let undo = apply(t.id, ultimate);
            reboot |= t.reboot;
            backup.insert(t.id.to_string(), undo);
            save_backup(&backup);
        }
    }
    reboot
}

pub fn revert_one(id: &str) -> bool {
    let mut backup = load_backup();
    if let Some(undo) = backup.remove(id) {
        revert(&undo);
        save_backup(&backup);
        return get(id).is_some_and(|t| t.reboot);
    }
    false
}

pub fn revert_all() -> bool {
    let backup = load_backup();
    let mut reboot = false;
    let order: Vec<&str> = TWEAKS.iter().rev().map(|t| t.id).collect();
    for id in order {
        if let Some(undo) = backup.get(id) {
            revert(undo);
            reboot |= get(id).is_some_and(|t| t.reboot);
        }
    }
    save_backup(&HashMap::new());
    reboot
}

pub fn applied_count() -> usize {
    load_backup().len()
}

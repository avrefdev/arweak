use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Output};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn run(program: &str, raw_args: &str) -> std::io::Result<Output> {
    Command::new(program)
        .raw_arg(raw_args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

pub fn stdout(program: &str, raw_args: &str) -> String {
    run(program, raw_args)
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

pub fn ps(script: &str) -> String {
    let out = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}"))
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
    let dir = PathBuf::from(base).join("Arweak");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn env_path(var: &str) -> PathBuf {
    PathBuf::from(std::env::var(var).unwrap_or_default())
}

pub fn reg_get(key: &str, name: &str) -> Option<(String, String)> {
    let out = stdout("reg.exe", &format!(r#"query "{key}" /v "{name}""#));
    out.lines().find_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix(name)?.trim_start();
        let mut it = rest.splitn(2, char::is_whitespace);
        let kind = it.next()?.to_string();
        if !kind.starts_with("REG_") {
            return None;
        }
        let data = it.next().unwrap_or("").trim().to_string();
        Some((kind, data))
    })
}

pub fn reg_set(key: &str, name: &str, kind: &str, data: &str) -> bool {
    let data = if kind == "REG_DWORD" {
        data.strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .map(|v| v.to_string())
            .unwrap_or_else(|| data.to_string())
    } else {
        data.to_string()
    };
    run("reg.exe", &format!(r#"add "{key}" /v "{name}" /t {kind} /d "{data}" /f"#))
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn reg_del(key: &str, name: &str) {
    let _ = run("reg.exe", &format!(r#"delete "{key}" /v "{name}" /f"#));
}

pub fn reg_subkeys(key: &str) -> Vec<String> {
    let prefix = key.to_lowercase();
    stdout("reg.exe", &format!(r#"query "{key}""#))
        .lines()
        .map(str::trim)
        .filter(|l| l.to_lowercase().starts_with(&prefix) && l.len() > key.len())
        .map(String::from)
        .collect()
}

pub struct Proc {
    pub pid: u32,
    pub name: String,
}

pub fn processes() -> Vec<Proc> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let mut list = Vec::new();
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return list;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut e) != 0 {
            loop {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
                list.push(Proc {
                    pid: e.th32ProcessID,
                    name: String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase(),
                });
                if Process32NextW(snap, &mut e) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    list
}

pub fn set_priority(pid: u32, class: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, SetPriorityClass, PROCESS_SET_INFORMATION,
    };
    unsafe {
        let h = OpenProcess(PROCESS_SET_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let ok = SetPriorityClass(h, class) != 0;
        CloseHandle(h);
        ok
    }
}

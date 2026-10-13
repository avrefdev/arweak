use crate::{games, memory, util};
use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PRESENTMON: &[u8] = include_bytes!("../tools/PresentMon.exe");
const HIGH_PRIORITY_CLASS: u32 = 0x80;
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x4000;
const NORMAL_PRIORITY_CLASS: u32 = 0x20;
const BACKGROUND: &[&str] = &[
    "chrome.exe", "msedge.exe", "firefox.exe", "opera.exe", "opera_gx.exe", "brave.exe",
    "onedrive.exe", "spotify.exe", "epicgameslauncher.exe", "steamwebhelper.exe",
];

#[derive(Default)]
pub struct Live {
    pub cpu: u32,
    pub gpu: u32,
    pub ram: u32,
    pub game: Option<games::Running>,
    pub frames: VecDeque<(Instant, f32)>,
    pub history: VecDeque<f32>,
    pub session: Vec<u32>,
    pub session_boost: Vec<bool>,
}

pub type Shared = Arc<Mutex<Live>>;

pub struct Options {
    pub boost: bool,
    pub prio: bool,
    pub bg: bool,
    pub ram: bool,
}

fn filetime(ft: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64
}

struct Cpu {
    last: (u64, u64),
}

impl Cpu {
    fn times() -> (u64, u64) {
        use windows_sys::Win32::System::Threading::GetSystemTimes;
        unsafe {
            let (mut i, mut k, mut u) = std::mem::zeroed();
            GetSystemTimes(&mut i, &mut k, &mut u);
            (filetime(i), filetime(k) + filetime(u))
        }
    }
    fn sample(&mut self) -> u32 {
        let now = Self::times();
        let idle = now.0.saturating_sub(self.last.0) as f64;
        let total = now.1.saturating_sub(self.last.1) as f64;
        self.last = now;
        if total <= 0.0 { 0 } else { ((1.0 - idle / total) * 100.0).clamp(0.0, 100.0).round() as u32 }
    }
}

struct Gpu {
    query: isize,
    counter: isize,
}

impl Gpu {
    fn new() -> Option<Self> {
        use windows_sys::Win32::System::Performance::{PdhAddEnglishCounterW, PdhCollectQueryData, PdhOpenQueryW};
        let path: Vec<u16> = r"\GPU Engine(*engtype_3D)\Utilization Percentage".encode_utf16().chain(Some(0)).collect();
        unsafe {
            let mut q = std::mem::zeroed();
            if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
                return None;
            }
            let mut c = std::mem::zeroed();
            if PdhAddEnglishCounterW(q, path.as_ptr(), 0, &mut c) != 0 {
                return None;
            }
            PdhCollectQueryData(q);
            Some(Self { query: q as isize, counter: c as isize })
        }
    }
    fn sample(&mut self) -> u32 {
        use windows_sys::Win32::System::Performance::{
            PdhCollectQueryData, PdhGetFormattedCounterArrayW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        };
        unsafe {
            if PdhCollectQueryData(self.query as _) != 0 {
                return 0;
            }
            let (mut size, mut count) = (0u32, 0u32);
            PdhGetFormattedCounterArrayW(self.counter as _, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut());
            if size == 0 {
                return 0;
            }
            let mut buf = vec![0u8; size as usize];
            let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(self.counter as _, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                return 0;
            }
            let slice = std::slice::from_raw_parts(items, count as usize);
            let sum: f64 = slice.iter().map(|i| i.FmtValue.Anonymous.doubleValue).sum();
            sum.clamp(0.0, 100.0).round() as u32
        }
    }
}

fn presentmon_path() -> std::path::PathBuf {
    let dir = util::data_dir().join("tools");
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("PresentMon.exe");
    if std::fs::metadata(&p).map(|m| m.len() != PRESENTMON.len() as u64).unwrap_or(true) {
        let _ = std::fs::write(&p, PRESENTMON);
    }
    p
}

fn start_presentmon(pid: u32, live: Shared) -> Option<Child> {
    let mut child = Command::new(presentmon_path())
        .args([
            "--process_id", &pid.to_string(), "--output_stdout", "--no_console_stats", "--v1_metrics",
            "--stop_existing_session", "--session_name", "Arweak", "--terminate_on_proc_exit",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(util::CREATE_NO_WINDOW)
        .spawn()
        .ok()?;
    let out = child.stdout.take()?;
    std::thread::spawn(move || {
        let mut col = None;
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            let cells: Vec<&str> = line.split(',').collect();
            match col {
                None => col = cells.iter().position(|c| matches!(c.trim().to_lowercase().as_str(), "msbetweenpresents" | "frametime")),
                Some(i) => {
                    if let Some(v) = cells.get(i).and_then(|v| v.trim().parse::<f32>().ok()) {
                        if v > 0.0 && v < 1000.0 {
                            let mut l = live.lock().unwrap();
                            let now = Instant::now();
                            l.frames.push_back((now, v));
                            while l.frames.front().is_some_and(|(t, _)| now - *t > Duration::from_secs(60)) {
                                l.frames.pop_front();
                            }
                        }
                    }
                }
            }
        }
    });
    Some(child)
}

pub fn fps_now(l: &Live) -> Option<u32> {
    let now = Instant::now();
    let recent: Vec<f32> = l.frames.iter().filter(|(t, _)| now - *t < Duration::from_secs(1)).map(|(_, v)| *v).collect();
    if recent.len() < 2 {
        return None;
    }
    let avg = recent.iter().sum::<f32>() / recent.len() as f32;
    Some((1000.0 / avg).round() as u32)
}

pub fn low_1(l: &Live) -> Option<u32> {
    let mut v: Vec<f32> = l.frames.iter().map(|(_, v)| *v).collect();
    if v.len() < 100 {
        return None;
    }
    v.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let p = v[v.len() / 100];
    Some((1000.0 / p).round() as u32)
}

pub fn stutters(l: &Live) -> Option<u32> {
    let mut v: Vec<f32> = l.frames.iter().map(|(_, v)| *v).collect();
    if v.len() < 100 {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = v[v.len() / 2];
    Some(v.iter().filter(|x| **x > median * 2.5 && **x > 20.0).count() as u32)
}

pub fn spawn(live: Shared, options: impl Fn() -> Options + Send + 'static, on_game: impl Fn(Option<&games::Running>, Option<(&'static str, u32, bool)>) + Send + 'static, enabled: impl Fn(&str) -> bool + Send + 'static) {
    std::thread::spawn(move || {
        let mut cpu = Cpu { last: Cpu::times() };
        let mut gpu = Gpu::new();
        let mut pm: Option<Child> = None;
        let mut lowered: HashSet<u32> = HashSet::new();
        let mut boosted_pid = 0u32;
        let mut last_purge = Instant::now() - Duration::from_secs(600);
        let mut tick = 0u64;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            tick += 1;
            let opt = options();
            let procs = util::processes();
            let game = games::running(&procs, &enabled);
            let c = cpu.sample();
            let g = gpu.as_mut().map(|g| g.sample()).unwrap_or(0);
            let mem = memory::status();

            let changed = {
                let mut l = live.lock().unwrap();
                l.cpu = c;
                l.gpu = g;
                l.ram = mem.load;
                let changed = l.game.as_ref().map(|x| x.pid) != game.as_ref().map(|x| x.pid);
                let ended = if changed {
                    let prev = l.game.take();
                    let summary = prev.and_then(|p| {
                        (l.session.len() >= 60).then(|| {
                            let avg = l.session.iter().sum::<u32>() / l.session.len() as u32;
                            let on = l.session_boost.iter().filter(|b| **b).count() * 2 > l.session_boost.len();
                            (p.id, avg, on)
                        })
                    });
                    l.frames.clear();
                    l.history.clear();
                    l.session.clear();
                    l.session_boost.clear();
                    l.game = game.clone();
                    summary
                } else {
                    None
                };
                if l.game.is_some() {
                    let now = Instant::now();
                    let last: Vec<f32> = l.frames.iter().filter(|(t, _)| now - *t < Duration::from_secs(1)).map(|(_, v)| *v).collect();
                    if !last.is_empty() {
                        let avg = last.iter().sum::<f32>() / last.len() as f32;
                        l.history.push_back(avg);
                        if l.history.len() > 60 {
                            l.history.pop_front();
                        }
                        l.session.push((1000.0 / avg).round() as u32);
                        l.session_boost.push(opt.boost);
                    }
                }
                changed.then_some(ended)
            };

            if let Some(ended) = changed {
                if let Some(mut child) = pm.take() {
                    let _ = child.kill();
                }
                if let Some(gm) = &game {
                    pm = start_presentmon(gm.pid, live.clone());
                }
                on_game(game.as_ref(), ended);
                boosted_pid = 0;
            }

            match (&game, opt.boost) {
                (Some(gm), true) => {
                    if opt.prio && boosted_pid != gm.pid && util::set_priority(gm.pid, HIGH_PRIORITY_CLASS) {
                        boosted_pid = gm.pid;
                    }
                    if opt.bg && tick % 5 == 0 {
                        let targets: Vec<u32> = procs
                            .iter()
                            .filter(|p| BACKGROUND.contains(&p.name.as_str()) && !lowered.contains(&p.pid))
                            .map(|p| p.pid)
                            .collect();
                        for pid in targets {
                            if util::set_priority(pid, BELOW_NORMAL_PRIORITY_CLASS) {
                                lowered.insert(pid);
                            }
                        }
                    }
                    let low_mem = mem.avail < 2 * 1024 * 1024 * 1024 || mem.load >= 85;
                    if opt.ram && low_mem && last_purge.elapsed() > Duration::from_secs(90) {
                        memory::clean();
                        last_purge = Instant::now();
                    }
                }
                _ => {
                    for pid in lowered.drain() {
                        util::set_priority(pid, NORMAL_PRIORITY_CLASS);
                    }
                }
            }
            if !opt.bg {
                for pid in lowered.drain() {
                    util::set_priority(pid, NORMAL_PRIORITY_CLASS);
                }
            }
        }
    });
}

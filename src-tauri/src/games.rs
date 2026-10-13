use crate::util;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Game {
    pub id: &'static str,
    pub name: &'static str,
    pub img: &'static str,
    pub exes: &'static [&'static str],
    source: Source,
}

enum Source {
    Steam(&'static str, &'static str),
    Epic(&'static str, &'static str),
    Riot(&'static str, &'static str),
    Minecraft,
}

pub const GAMES: &[Game] = &[
    Game {
        id: "fortnite",
        name: "Fortnite",
        img: "fortnite.jpg",
        exes: &["fortniteclient-win64-shipping.exe"],
        source: Source::Epic("Fortnite", r"FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe"),
    },
    Game {
        id: "valorant",
        name: "Valorant",
        img: "valorant.jpg",
        exes: &["valorant-win64-shipping.exe"],
        source: Source::Riot("valorant.live", r"ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe"),
    },
    Game {
        id: "cs2",
        name: "Counter-Strike 2",
        img: "cs2.jpg",
        exes: &["cs2.exe"],
        source: Source::Steam("730", r"game\bin\win64\cs2.exe"),
    },
    Game {
        id: "apex",
        name: "Apex Legends",
        img: "apex.jpg",
        exes: &["r5apex.exe", "r5apex_dx12.exe"],
        source: Source::Steam("1172470", "r5apex.exe"),
    },
    Game {
        id: "lol",
        name: "League of Legends",
        img: "lol.jpg",
        exes: &["league of legends.exe"],
        source: Source::Riot("league_of_legends.live", r"Game\League of Legends.exe"),
    },
    Game {
        id: "minecraft",
        name: "Minecraft",
        img: "minecraft.jpg",
        exes: &["javaw.exe", "minecraft.windows.exe"],
        source: Source::Minecraft,
    },
    Game {
        id: "gta",
        name: "GTA V",
        img: "gta.jpg",
        exes: &["gta5.exe", "gta5_enhanced.exe"],
        source: Source::Steam("271590", "GTA5.exe"),
    },
    Game {
        id: "rl",
        name: "Rocket League",
        img: "rl.jpg",
        exes: &["rocketleague.exe"],
        source: Source::Steam("252950", r"Binaries\Win64\RocketLeague.exe"),
    },
];

pub fn by_id(id: &str) -> Option<&'static Game> {
    GAMES.iter().find(|g| g.id == id)
}

fn steam_libraries() -> Vec<PathBuf> {
    let Some((_, root)) = util::reg_get(r"HKCU\Software\Valve\Steam", "SteamPath") else {
        return Vec::new();
    };
    let root = PathBuf::from(root.replace('/', "\\"));
    let mut libs = vec![root.clone()];
    if let Ok(vdf) = std::fs::read_to_string(root.join(r"steamapps\libraryfolders.vdf")) {
        for line in vdf.lines() {
            let parts: Vec<&str> = line.split('"').filter(|s| !s.trim().is_empty()).collect();
            if parts.len() == 2 && parts[0] == "path" {
                libs.push(PathBuf::from(parts[1].replace("\\\\", "\\")));
            }
        }
    }
    libs
}

fn acf_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let parts: Vec<&str> = l.split('"').filter(|s| !s.trim().is_empty()).collect();
        (parts.len() == 2 && parts[0].eq_ignore_ascii_case(key)).then(|| parts[1].to_string())
    })
}

fn steam_dir(app: &str) -> Option<PathBuf> {
    steam_libraries().into_iter().find_map(|lib| {
        let acf = std::fs::read_to_string(lib.join(format!(r"steamapps\appmanifest_{app}.acf"))).ok()?;
        let dir = acf_value(&acf, "installdir")?;
        Some(lib.join(r"steamapps\common").join(dir))
    })
}

fn epic_dir(name: &str) -> Option<PathBuf> {
    let manifests = util::env_path("ProgramData").join(r"Epic\EpicGamesLauncher\Data\Manifests");
    std::fs::read_dir(manifests).ok()?.flatten().find_map(|e| {
        let text = std::fs::read_to_string(e.path()).ok()?;
        let json: serde_json::Value = serde_json::from_str(&text).ok()?;
        let display = json.get("DisplayName")?.as_str()?;
        if !display.eq_ignore_ascii_case(name) {
            return None;
        }
        Some(PathBuf::from(json.get("InstallLocation")?.as_str()?))
    })
}

fn riot_dir(product: &str) -> Option<PathBuf> {
    let meta = util::env_path("ProgramData").join(r"Riot Games\Metadata").join(product);
    if !meta.exists() {
        return None;
    }
    let settings = std::fs::read_to_string(meta.join(format!("{product}.product_settings.yaml"))).ok();
    if let Some(root) = settings.and_then(|s| {
        s.lines().find_map(|l| {
            l.trim()
                .strip_prefix("product_install_full_path:")
                .map(|v| v.trim().trim_matches('"').replace('/', "\\"))
        })
    }) {
        return Some(PathBuf::from(root));
    }
    let guess = if product.starts_with("valorant") {
        r"C:\Riot Games\VALORANT\live"
    } else {
        r"C:\Riot Games\League of Legends"
    };
    Some(PathBuf::from(guess))
}

pub fn exe_path(game: &Game) -> Option<PathBuf> {
    let (dir, rel) = match &game.source {
        Source::Steam(app, rel) => (steam_dir(app)?, *rel),
        Source::Epic(name, rel) => (epic_dir(name)?, *rel),
        Source::Riot(product, rel) => (riot_dir(product)?, *rel),
        Source::Minecraft => return None,
    };
    Some(dir.join(rel))
}

pub fn installed(game: &Game) -> bool {
    match &game.source {
        Source::Minecraft => {
            util::env_path("APPDATA").join(".minecraft").exists()
                || Path::new(r"C:\XboxGames\Minecraft Launcher").exists()
        }
        Source::Riot(product, _) => util::env_path("ProgramData")
            .join(r"Riot Games\Metadata")
            .join(product)
            .exists(),
        _ => exe_path(game).is_some_and(|p| p.exists() || p.parent().is_some_and(Path::exists)),
    }
}

#[derive(Serialize, Clone)]
pub struct Running {
    pub id: &'static str,
    pub pid: u32,
    pub exe: String,
}

pub fn running(procs: &[util::Proc], enabled: impl Fn(&str) -> bool) -> Option<Running> {
    GAMES.iter().filter(|g| enabled(g.id)).find_map(|g| {
        if g.id == "minecraft" && !installed(g) {
            return None;
        }
        procs.iter().find(|p| g.exes.contains(&p.name.as_str())).map(|p| Running {
            id: g.id,
            pid: p.pid,
            exe: p.name.clone(),
        })
    })
}

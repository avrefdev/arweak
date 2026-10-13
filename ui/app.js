"use strict";

const tauri = window.__TAURI__;
const invoke = tauri ? tauri.core.invoke : mockInvoke;
const $ = (id) => document.getElementById(id);
const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

let page = "home";
let state = null;
let system = null;
let lastImg = "";
const FOCUS = { "valorant.jpg": "78% 35%", "cs2.jpg": "68% 30%", "apex.jpg": "60% 30%", "fortnite.jpg": "50% 30%", "gta.jpg": "45% 35%" };
const focus = (img) => FOCUS[img] || "50% 50%";

const nav = [...document.querySelectorAll(".nv[data-p]")];
function moveIndicator(btn) {
  $("nav-ind").style.transform = `translateY(${btn.offsetTop}px)`;
}

function go(p) {
  if (p === page) return;
  page = p;
  nav.forEach((b) => b.classList.toggle("a", b.dataset.p === p));
  moveIndicator(nav.find((b) => b.dataset.p === p));
  document.querySelectorAll(".page").forEach((s) => s.classList.toggle("active", s.id === `page-${p}`));
  if (p === "games") loadGames();
  if (p === "tweaks") loadTweaks();
  if (p === "system") loadSystem(false);
  if (p === "settings") loadSettings();
  render();
}
nav.forEach((b) => b.addEventListener("click", () => go(b.dataset.p)));
moveIndicator(nav[0]);

$("win-min").addEventListener("click", () => tauri?.window.getCurrentWindow().minimize());
$("win-close").addEventListener("click", () => invoke("close_window"));
document.addEventListener("contextmenu", (e) => e.preventDefault());

function gauge(el, label, value, unit) {
  if (!el.firstChild) {
    el.innerHTML = `<div class="gauge"><svg viewBox="0 0 80 80"><circle cx="40" cy="40" r="34" class="track"/><circle cx="40" cy="40" r="34" class="val"/></svg><div class="num d"></div></div><div class="lb">${label}</div>`;
  }
  const v = typeof value === "number" ? value : null;
  const ring = el.querySelector(".val");
  ring.style.strokeDashoffset = 213.6 * (1 - (v ?? 0) / 100);
  ring.style.stroke = v != null && v > 85 ? "var(--wr)" : "var(--ac)";
  el.querySelector(".num").innerHTML = v == null ? "—" : `${v}<small>${unit}</small>`;
}

function chart(svg, values) {
  if (!svg.querySelector(".line")) {
    svg.innerHTML = `<defs><linearGradient id="fill" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#2fbf7f" stop-opacity=".22"/><stop offset="1" stop-color="#2fbf7f" stop-opacity="0"/></linearGradient></defs><line x1="0" y1="59.5" x2="100" y2="59.5" class="axis"/><polygon class="area"/><polyline class="line"/>`;
  }
  if (values.length < 2) {
    svg.querySelector(".line").setAttribute("points", "");
    svg.querySelector(".area").setAttribute("points", "");
    return;
  }
  const max = Math.max(...values) * 1.2;
  const min = Math.max(0, Math.min(...values) * 0.8);
  const span = Math.max(max - min, 2);
  const step = 100 / (values.length - 1);
  const pts = values.map((v, i) => `${(i * step).toFixed(2)},${(58 - ((v - min) / span) * 54).toFixed(2)}`).join(" ");
  svg.querySelector(".line").setAttribute("points", pts);
  svg.querySelector(".area").setAttribute("points", `0,60 ${pts} 100,60`);
}

let shownScore = 0;
function animateScore(target) {
  const start = shownScore;
  const t0 = performance.now();
  const step = (t) => {
    const k = Math.min(1, (t - t0) / 700);
    const e = 1 - Math.pow(1 - k, 3);
    shownScore = Math.round(start + (target - start) * e);
    $("score").textContent = shownScore;
    if (k < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
  const color = target >= 80 ? "var(--ac)" : "var(--wr)";
  $("score").style.color = color;
  $("score-bar").style.width = `${target}%`;
  $("score-bar").style.background = color;
}

function setText(id, v) {
  const el = $(id);
  const s = v == null ? "—" : String(v);
  if (el.textContent !== s) el.textContent = s;
}

function render() {
  if (!state) return;
  const s = state;
  const b = $("boost");
  b.classList.toggle("on", s.boost && !s.busy);
  b.classList.toggle("busy", s.busy);
  $("boost-label").textContent = s.busy ? "UYGULANIYOR" : s.boost ? "AKTİF" : "BOOST";
  $("boost-title").textContent = s.boost ? `${s.active} optimizasyon aktif` : "Optimize edilmedi";
  $("boost-sub").textContent = s.reboot ? "Bazı ayarlar için yeniden başlatın" : s.boost && s.game.running ? `${s.game.name} algılandı` : "";
  $("hdr").textContent = s.boost ? "BOOST AKTİF" : "HAZIR";

  if (s.game.img !== lastImg) {
    const img = $("hero-img");
    img.classList.add("swap");
    setTimeout(() => {
      img.style.backgroundImage = `url("img/${s.game.img}")`;
      img.style.backgroundPosition = focus(s.game.img);
      img.classList.remove("swap");
    }, lastImg ? 250 : 0);
    lastImg = s.game.img;
  }
  setText("hero-name", s.game.name);
  $("hero-tag").innerHTML = s.game.running ? '<span class="live-dot"></span>ŞU AN OYNANIYOR' : "SEÇİLİ PROFİL";
  setText("hero-fps", s.fps);
  $("hero-fps").style.color = s.boost && s.fps ? "var(--ac)" : "#fff";

  if (s.score != null && s.score !== shownScore) animateScore(s.score);

  if (page === "home") {
    gauge(document.querySelector('[data-ring="cpu"]'), "İşlemci", s.cpu, "%");
    gauge(document.querySelector('[data-ring="gpu"]'), "Ekran kartı", s.gpu, "%");
    gauge(document.querySelector('[data-ring="ram"]'), "RAM", s.ram, "%");
    $("home-low").textContent = s.low ? `%1 low · ${s.low} FPS` : "";
    chart($("home-chart"), s.history);
  }
  if (page === "perf") {
    setText("perf-game", s.game.running ? s.game.name : "Oyun bekleniyor");
    setText("p-fps", s.fps);
    setText("p-low", s.low);
    setText("p-ft", s.frametime != null ? s.frametime.toFixed(1) : null);
    setText("p-st", s.stutters);
    chart($("perf-chart"), s.history);
    setText("p-before", s.before);
    setText("p-after", s.after);
    $("p-gain").textContent = s.before && s.after ? `${s.after >= s.before ? "+" : ""}${Math.round((s.after / s.before - 1) * 100)}%` : "";
  }
  if (page === "tweaks") {
    $("reboot-note").hidden = !s.reboot;
  }
}

async function poll() {
  try {
    state = await invoke("get_state");
  } catch (_) {}
  render();
}

$("boost").addEventListener("click", async () => {
  if (!state || state.busy) return;
  state.busy = true;
  render();
  try {
    await invoke("toggle_boost");
  } catch (_) {}
  await poll();
  if (state?.score == null) loadSystem(false);
});

async function loadGames() {
  const list = await invoke("get_games");
  $("games-count").textContent = `${list.filter((g) => g.profile).length} profil aktif`;
  $("games").innerHTML = list
    .map(
      (g, i) => `<div class="gm st-i ${g.selected ? "s" : ""} ${g.installed ? "" : "off"}" style="--i:${i}" data-id="${g.id}">
        <div class="ban"><div class="ban-img" style="background-image:url('img/${g.img}');background-position:${focus(g.img)}"></div><input type="checkbox" class="tg" data-profile="${g.id}" ${g.profile ? "checked" : ""}></div>
        <div class="gm-info"><b>${esc(g.name)}</b><small class="${g.installed ? "t-ok" : "muted"}">${g.installed ? "YÜKLÜ" : "YOK"}</small></div>
      </div>`,
    )
    .join("");
}
$("games").addEventListener("click", async (e) => {
  const tg = e.target.closest("[data-profile]");
  if (tg) {
    e.stopPropagation();
    await invoke("set_profile", { id: tg.dataset.profile, on: tg.checked });
    const n = document.querySelectorAll("[data-profile]:checked").length;
    $("games-count").textContent = `${n} profil aktif`;
    return;
  }
  const card = e.target.closest(".gm");
  if (!card) return;
  document.querySelectorAll(".gm").forEach((c) => c.classList.toggle("s", c === card));
  await invoke("select_game", { id: card.dataset.id });
  poll();
});

async function loadTweaks() {
  const list = await invoke("get_tweaks");
  const groups = [...new Set(list.map((t) => t.group))];
  const icons = { Sistem: "ti-device-desktop", "Ekran kartı": "ti-cpu-2", "Bellek ve ağ": "ti-wifi" };
  $("tweaks").innerHTML = groups
    .map(
      (g, gi) => `<div class="col"><p class="cat st-i" style="--i:${gi}"><i class="ti ${icons[g] || "ti-bolt"}"></i>${g}</p><div class="stack">${list
        .filter((t) => t.group === g)
        .map((t, i) => `<label class="bx st-i" style="--i:${gi + i + 1}">${esc(t.name)}<input type="checkbox" class="tg" data-tweak="${t.id}" ${t.on ? "checked" : ""}></label>`)
        .join("")}</div></div>`,
    )
    .join("");
  updateTweakCount();
}
function updateTweakCount() {
  const all = document.querySelectorAll("[data-tweak]");
  const on = document.querySelectorAll("[data-tweak]:checked").length;
  $("tw-count").textContent = `${on} / ${all.length}`;
}
$("tweaks").addEventListener("change", async (e) => {
  const t = e.target.closest("[data-tweak]");
  if (!t) return;
  updateTweakCount();
  await invoke("set_tweak", { id: t.dataset.tweak, on: t.checked });
  poll();
});
for (const [id, rec] of [["tw-rec", true], ["tw-off", false]]) {
  $(id).addEventListener("click", async () => {
    $(id).classList.add("busy");
    await invoke("set_all_tweaks", { recommended: rec });
    $(id).classList.remove("busy");
    await loadTweaks();
    poll();
  });
}

function rows(list) {
  return list.map((r) => `<div class="bx"><span class="k">${esc(r.name)}</span><span class="v t-${r.tone}">${esc(r.value)}</span></div>`).join("");
}
async function loadSystem(fresh) {
  if (!system) {
    const wait = (names) => names.map((name) => ({ name, value: "Taranıyor", tone: "wait" }));
    $("sys-hw").innerHTML = rows(wait(["İşlemci", "Ekran kartı", "RAM", "Sürücü"]));
    $("sys-mem").innerHTML = rows(wait(["Kanal", "Hız", "EXPO / XMP", "Boş bellek"]));
    $("sys-disp").innerHTML = rows(wait(["Yenileme hızı", "Oyun Modu", "HAGS", "Güç planı"]));
  }
  $("sys-refresh").classList.add("busy");
  system = await invoke("get_system", { fresh });
  $("sys-refresh").classList.remove("busy");
  $("sys-hw").innerHTML = rows(system.hardware);
  $("sys-mem").innerHTML = rows(system.memory);
  $("sys-disp").innerHTML = rows(system.display);
  $("sys-warn").hidden = !system.warning;
  $("sys-warn-text").textContent = system.warning || "";
  poll();
}
$("sys-refresh").addEventListener("click", () => loadSystem(true));
$("clean").addEventListener("click", async () => {
  const btn = $("clean");
  btn.classList.add("busy");
  btn.innerHTML = '<i class="ti ti-loader-2"></i>Temizleniyor';
  const mb = await invoke("clean_ram");
  btn.classList.remove("busy");
  btn.innerHTML = '<i class="ti ti-check"></i>Temizlendi';
  const r = $("clean-result");
  r.textContent = mb >= 1024 ? `${(mb / 1024).toFixed(1).replace(".", ",")} GB boşaldı` : `${mb} MB boşaldı`;
  r.classList.remove("flash");
  void r.offsetWidth;
  r.classList.add("flash");
  setTimeout(() => (btn.innerHTML = '<i class="ti ti-eraser"></i>Temizle'), 2500);
  loadSystem(true);
});

async function loadSettings() {
  const s = await invoke("get_settings");
  document.querySelectorAll("[data-set]").forEach((t) => (t.checked = !!s[t.dataset.set]));
}
document.querySelectorAll("[data-set]").forEach((t) =>
  t.addEventListener("change", () => invoke("set_setting", { key: t.dataset.set, value: t.checked })),
);
$("restore").addEventListener("click", async () => {
  const b = $("restore");
  b.textContent = "Oluşturuluyor…";
  const ok = await invoke("restore_point");
  b.textContent = ok ? "Oluşturuldu" : "Oluşturulamadı";
  setTimeout(() => (b.textContent = "Oluştur"), 3000);
});
let confirmTimer;
$("revert").addEventListener("click", async () => {
  const b = $("revert");
  if (!b.dataset.confirm) {
    b.dataset.confirm = "1";
    b.textContent = "Emin misiniz?";
    clearTimeout(confirmTimer);
    confirmTimer = setTimeout(() => { delete b.dataset.confirm; b.textContent = "Geri al"; }, 3000);
    return;
  }
  delete b.dataset.confirm;
  b.textContent = "Geri alınıyor…";
  await invoke("revert_everything");
  b.textContent = "Geri alındı";
  setTimeout(() => (b.textContent = "Geri al"), 2500);
  poll();
});

poll();
loadSystem(false);
setInterval(poll, 1000);

function mockInvoke(cmd, args) {
  const m = (window.__m ??= {
    boost: false, busy: false, game: "fortnite", running: false, hist: [], t0: 0,
    tweaks: { power: true, gamebar: true, prio: true, bg: false, gpu: true, gamemode: true, hags: true, mpo: false, ram: true, net: true, nagle: false, timer: true },
    profiles: {}, settings: { start_with_windows: true, auto_boost: true, close_to_tray: true },
  });
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const G = [["fortnite", "Fortnite", "fortnite.jpg", 1], ["valorant", "Valorant", "valorant.jpg", 1], ["cs2", "Counter-Strike 2", "cs2.jpg", 1], ["apex", "Apex Legends", "apex.jpg", 0], ["lol", "League of Legends", "lol.jpg", 1], ["minecraft", "Minecraft", "minecraft.jpg", 1], ["gta", "GTA V", "gta.jpg", 0], ["rl", "Rocket League", "rl.jpg", 0]];
  const T = [["power", "Ultimate güç planı", "Sistem"], ["gamebar", "Game Bar kaydını kapat", "Sistem"], ["prio", "Oyuna yüksek öncelik", "Sistem"], ["bg", "Oyunda arka planı kıs", "Sistem"], ["gpu", "GPU maksimum performans", "Ekran kartı"], ["gamemode", "Windows Oyun Modu", "Ekran kartı"], ["hags", "Donanım hızlandırmalı zamanlama", "Ekran kartı"], ["mpo", "Çoklu düzlem kaplamasını kapat", "Ekran kartı"], ["ram", "Akıllı RAM temizleme", "Bellek ve ağ"], ["net", "Ağ güç tasarrufunu kapat", "Bellek ve ağ"], ["nagle", "Nagle algoritmasını kapat", "Bellek ve ağ"], ["timer", "Yüksek çözünürlüklü zamanlayıcı", "Bellek ve ağ"]];
  const active = () => (m.boost ? Object.values(m.tweaks).filter(Boolean).length : 0);
  switch (cmd) {
    case "get_state": {
      const g = G.find((x) => x[0] === m.game);
      if (m.running) {
        m.hist.push(4.2 + Math.random() * 0.5 + (Math.random() < 0.04 ? 2.2 : 0));
        if (m.hist.length > 60) m.hist.shift();
      }
      const avg = m.hist.length ? m.hist.reduce((a, b) => a + b, 0) / m.hist.length : 0;
      const fps = m.running && avg ? Math.round(1000 / avg) : null;
      return Promise.resolve({
        boost: m.boost, busy: m.busy, reboot: false, active: active(), total: 12,
        score: Math.round(40 + 10 + 35 * active() / 12),
        game: { id: g[0], name: g[1], img: g[2], running: m.running },
        fps, low: m.hist.length > 20 ? Math.round(1000 / Math.max(...m.hist)) : null,
        stutters: m.running ? m.hist.filter((v) => v > 6).length : null,
        frametime: m.hist.at(-1) ?? null, history: m.hist,
        cpu: m.running ? Math.round(60 + Math.random() * 14) : 9, gpu: m.running ? Math.round(80 + Math.random() * 12) : 4, ram: m.running ? 72 : 58,
        before: 162, after: fps,
      });
    }
    case "toggle_boost":
      m.busy = true;
      return wait(1200).then(() => { m.busy = false; m.boost = !m.boost; m.running = m.boost; m.hist = []; });
    case "get_games":
      return Promise.resolve(G.map(([id, name, img, inst]) => ({ id, name, img, installed: !!inst, profile: m.profiles[id] ?? !!inst, selected: m.game === id })));
    case "set_profile": m.profiles[args.id] = args.on; return Promise.resolve();
    case "select_game": m.game = args.id; return Promise.resolve();
    case "get_tweaks": return Promise.resolve(T.map(([id, name, group]) => ({ id, name, group, on: !!m.tweaks[id] })));
    case "set_tweak": m.tweaks[args.id] = args.on; return wait(150);
    case "set_all_tweaks":
      Object.keys(m.tweaks).forEach((k) => (m.tweaks[k] = args.recommended && !["bg", "mpo", "nagle"].includes(k)));
      return wait(500);
    case "get_system":
      return wait(600).then(() => ({
        hardware: [{ name: "İşlemci", value: "AMD Ryzen 5 7600X", tone: "plain" }, { name: "Ekran kartı", value: "RTX 5060 Ti", tone: "plain" }, { name: "RAM", value: "16 GB", tone: "plain" }, { name: "Sürücü", value: "581.42", tone: "plain" }],
        memory: [{ name: "Kanal", value: "Tek kanal", tone: "err" }, { name: "Hız", value: "5200 / 5600", tone: "warn" }, { name: "EXPO / XMP", value: "Kapalı", tone: "warn" }, { name: "Boş bellek", value: "6,2 GB", tone: "plain" }],
        display: [{ name: "Yenileme hızı", value: "240 Hz", tone: "ok" }, { name: "Oyun Modu", value: "Açık", tone: "ok" }, { name: "HAGS", value: "Kapalı", tone: "warn" }, { name: "Güç planı", value: m.boost ? "Ultimate Performance" : "Dengeli", tone: m.boost ? "ok" : "warn" }],
        dual_channel: false, expo: false, warning: "İkinci bir RAM ile çift kanala geçin",
      }));
    case "clean_ram": return wait(900).then(() => 2380);
    case "get_settings": return Promise.resolve(m.settings);
    case "set_setting": m.settings[args.key] = args.value; return Promise.resolve();
    case "restore_point": return wait(1500).then(() => true);
    case "revert_everything": return wait(800).then(() => { m.boost = false; m.running = false; });
    default: return Promise.resolve();
  }
}

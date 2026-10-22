import { copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";

mkdirSync("ui/fonts", { recursive: true });
const inter = "node_modules/@fontsource-variable/inter/files";
for (const s of ["latin", "latin-ext"]) copyFileSync(`${inter}/inter-${s}-wght-normal.woff2`, `ui/fonts/inter-${s}.woff2`);
const chakra = "node_modules/@fontsource/chakra-petch/files";
for (const s of ["latin", "latin-ext"])
  for (const w of [500, 600, 700]) copyFileSync(`${chakra}/chakra-petch-${s}-${w}-normal.woff2`, `ui/fonts/chakra-${s}-${w}.woff2`);

const tabler = "node_modules/@tabler/icons-webfont/dist";
copyFileSync(`${tabler}/fonts/tabler-icons.woff2`, "ui/fonts/tabler-icons.woff2");
const css = readFileSync(`${tabler}/tabler-icons.min.css`, "utf8").replace(/url\([^)]*tabler-icons\.woff2[^)]*\)/g, 'url("fonts/tabler-icons.woff2")');
const used = ["bolt", "device-gamepad-2", "adjustments-horizontal", "chart-line", "cpu", "cpu-2", "settings", "minus", "x", "refresh", "checks", "device-desktop", "wifi", "database", "alert-triangle", "arrow-big-right", "loader-2", "eraser", "check", "rotate-2", "shield-check"];
const head = css.replace(/\/\*[\s\S]*?\*\//g, "").slice(0, css.replace(/\/\*[\s\S]*?\*\//g, "").indexOf(".ti-"));
const rules = used.map((n) => {
  const m = css.match(new RegExp(`\\.ti-${n}:before\\{content:"([^"]+)"\\}`));
  return m ? `.ti-${n}:before{content:"${m[1]}"}` : "";
});
writeFileSync("ui/icons.css", head.replace(/src:[^;}]+/, 'src:url("fonts/tabler-icons.woff2") format("woff2")') + rules.join(""));
console.log("varlıklar hazır");

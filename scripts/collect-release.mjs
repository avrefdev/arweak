import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";

const { version } = JSON.parse(readFileSync("package.json", "utf8"));
const target = "src-tauri/target/release";
mkdirSync("release", { recursive: true });
copyFileSync(`${target}/arweak.exe`, "release/Arweak.exe");
const setup = readdirSync(`${target}/bundle/nsis`).find((f) => f.endsWith("-setup.exe"));
if (setup) copyFileSync(`${target}/bundle/nsis/${setup}`, `release/Arweak_${version}_Setup.exe`);
if (existsSync("../exe")) copyFileSync(`${target}/arweak.exe`, "../exe/Arweak.exe");
console.log("release hazır");

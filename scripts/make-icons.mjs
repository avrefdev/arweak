import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
const crc = (buf) => {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
};
const chunk = (type, data) => {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type), data]);
  const c = Buffer.alloc(4);
  c.writeUInt32BE(crc(body));
  return Buffer.concat([len, body, c]);
};

function png(size, pixel) {
  const raw = Buffer.alloc(size * (size * 4 + 1));
  const ss = 4;
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < ss; sy++)
        for (let sx = 0; sx < ss; sx++) {
          const p = pixel((x + (sx + 0.5) / ss) / size, (y + (sy + 0.5) / ss) / size);
          r += p[0] * p[3]; g += p[1] * p[3]; b += p[2] * p[3]; a += p[3];
        }
      const o = y * (size * 4 + 1) + 1 + x * 4;
      raw[o] = a ? r / a : 0; raw[o + 1] = a ? g / a : 0; raw[o + 2] = a ? b / a : 0;
      raw[o + 3] = (a / (ss * ss)) * 255;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0); ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0)),
  ]);
}

function segDist(px, py, ax, ay, bx, by) {
  const vx = bx - ax, vy = by - ay;
  const t = Math.max(0, Math.min(1, ((px - ax) * vx + (py - ay) * vy) / (vx * vx + vy * vy)));
  return Math.hypot(px - (ax + vx * t), py - (ay + vy * t));
}

function glyph(u, v, scale, stroke) {
  const x = (u - 0.5) / scale + 0.5, y = (v - 0.5) / scale + 0.5;
  const left = segDist(x, y, 0.5, 0.14, 0.17, 0.84);
  const right = segDist(x, y, 0.5, 0.14, 0.83, 0.84);
  const bar = segDist(x, y, 0.06, 0.6, 0.66, 0.6);
  if (bar < stroke / 2) return 2;
  if (bar < stroke * 0.95) return 0;
  if (Math.min(left, right) < stroke / 2) return 1;
  return 0;
}

function roundedSquare(u, v, radius) {
  const dx = Math.max(Math.abs(u - 0.5) - (0.5 - radius), 0);
  const dy = Math.max(Math.abs(v - 0.5) - (0.5 - radius), 0);
  return Math.hypot(dx, dy) <= radius;
}

const BG = [18, 19, 24], FG = [244, 244, 245], AC = [47, 191, 127];

const app = png(1024, (u, v) => {
  if (!roundedSquare(u, v, 0.22)) return [0, 0, 0, 0];
  const g = glyph(u, v, 0.66, 0.12);
  return g === 1 ? [...FG, 1] : g === 2 ? [...AC, 1] : [...BG, 1];
});
writeFileSync("src-tauri/icons/app.png", app);

const tray = (fg, bar) =>
  png(32, (u, v) => {
    const g = glyph(u, v, 1, 0.16);
    return g === 1 ? [...fg, 1] : g === 2 ? [...bar, 1] : [0, 0, 0, 0];
  });
writeFileSync("src-tauri/icons/tray-off.png", tray([161, 161, 170], [161, 161, 170]));
writeFileSync("src-tauri/icons/tray-on.png", tray([244, 244, 245], AC));

const svgLogo = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><g fill="none" stroke-linecap="round" stroke-width="12"><path d="M50 14 17 84M50 14l33 70" stroke="#f4f4f5"/><path d="M6 60h60" stroke="#121318" stroke-width="22"/><path d="M6 60h60" stroke="#2fbf7f"/></g></svg>`;
writeFileSync("ui/img/logo.svg", svgLogo);
console.log("ikonlar üretildi");

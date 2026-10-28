const zlib = require("zlib");
const fs = require("fs");

const S = 1024;
const SS = 4; // supersampling per axis
const TILE = { x0: 64, y0: 64, x1: 960, y1: 960, r: 208 };
const DARK = [0x13, 0x14, 0x16];
const LIGHT = [0xf9, 0xfb, 0xfc];
const MUTED = [0xb9, 0xc0, 0xc3];

const cell = 176,
  gap = 44,
  pad = 140;
const start = TILE.x0 + pad;
const cells = [];
for (let r = 0; r < 3; r++) {
  for (let c = 0; c < 3; c++) {
    const muted = r === 1 && c === 2;
    cells.push({
      x0: start + c * (cell + gap),
      y0: start + r * (cell + gap),
      x1: start + c * (cell + gap) + cell,
      y1: start + r * (cell + gap) + cell,
      color: muted ? MUTED : DARK,
    });
  }
}

function inRounded(x, y, b) {
  if (x < b.x0 || x > b.x1 || y < b.y0 || y > b.y1) return false;
  const cx = Math.min(Math.max(x, b.x0 + b.r), b.x1 - b.r);
  const cy = Math.min(Math.max(y, b.y0 + b.r), b.y1 - b.r);
  const dx = x - cx,
    dy = y - cy;
  return dx * dx + dy * dy <= b.r * b.r;
}

const raw = Buffer.alloc((S * 4 + 1) * S);
const step = 1 / SS;
for (let y = 0; y < S; y++) {
  const rowStart = y * (S * 4 + 1);
  raw[rowStart] = 0;
  for (let x = 0; x < S; x++) {
    let r = 0,
      g = 0,
      b = 0,
      a = 0;
    for (let sy = 0; sy < SS; sy++) {
      for (let sx = 0; sx < SS; sx++) {
        const px = x + (sx + 0.5) * step;
        const py = y + (sy + 0.5) * step;
        if (!inRounded(px, py, TILE)) continue;
        let col = LIGHT;
        for (const c of cells) {
          if (px >= c.x0 && px <= c.x1 && py >= c.y0 && py <= c.y1) {
            col = c.color;
            break;
          }
        }
        r += col[0];
        g += col[1];
        b += col[2];
        a += 255;
      }
    }
    const n = SS * SS;
    const p = rowStart + 1 + x * 4;
    if (a === 0) {
      raw[p] = raw[p + 1] = raw[p + 2] = raw[p + 3] = 0;
      continue;
    }
    raw[p] = Math.round(r / (a / 255));
    raw[p + 1] = Math.round(g / (a / 255));
    raw[p + 2] = Math.round(b / (a / 255));
    raw[p + 3] = Math.round(a / n);
  }
}

let table = null;
function crc32(buf) {
  if (!table) {
    table = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      table[n] = c;
    }
  }
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = table[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const t = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([t, data])));
  return Buffer.concat([len, t, data, crc]);
}
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8;
ihdr[9] = 6;
const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);
fs.writeFileSync("design/app-icon.png", png);
console.log("png", png.length, "bytes");

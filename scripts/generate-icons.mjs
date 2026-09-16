// Generates every icon Tauri needs from one vector-ish description.
// Pure Node (zlib only) so `npm install` stays free of image dependencies.

import { deflateSync } from 'node:zlib'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const ICON_DIR = join(ROOT, 'src-tauri', 'icons')

// ---- mark -----------------------------------------------------------------------------

const TRACK = [0x2a, 0x33, 0x40]
const START = [0x4a, 0xde, 0x80] // codex green
const END = [0xf5, 0x9e, 0x6a] // claude amber

const SWEEP_START = (135 * Math.PI) / 180
const SWEEP_LENGTH = (270 * Math.PI) / 180
const FILLED = 0.68

const lerp = (a, b, t) => a + (b - a) * t
const mix = (a, b, t) => [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]

/** Color and coverage of the gauge at one point, in unit coordinates centred on (0,0). */
function sample(x, y, size) {
  const outer = 0.46
  const inner = size <= 32 ? 0.2 : 0.24
  const radius = Math.hypot(x, y)
  if (radius > outer || radius < inner) return null

  // Angle measured clockwise from the sweep start, in [0, 2pi).
  let angle = Math.atan2(y, x) - SWEEP_START
  while (angle < 0) angle += Math.PI * 2
  while (angle >= Math.PI * 2) angle -= Math.PI * 2
  if (angle > SWEEP_LENGTH) return null

  const t = angle / SWEEP_LENGTH
  return t <= FILLED ? mix(START, END, t / FILLED) : TRACK
}

/** Renders the mark into an RGBA buffer with 4x4 supersampling. */
function render(size) {
  const pixels = Buffer.alloc(size * size * 4)
  const SS = 4

  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let r = 0
      let g = 0
      let b = 0
      let hits = 0

      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const x = (px + (sx + 0.5) / SS) / size - 0.5
          const y = (py + (sy + 0.5) / SS) / size - 0.5
          const color = sample(x, y, size)
          if (!color) continue
          r += color[0]
          g += color[1]
          b += color[2]
          hits++
        }
      }

      const total = SS * SS
      const offset = (py * size + px) * 4
      if (hits === 0) continue
      pixels[offset] = Math.round(r / hits)
      pixels[offset + 1] = Math.round(g / hits)
      pixels[offset + 2] = Math.round(b / hits)
      pixels[offset + 3] = Math.round((hits / total) * 255)
    }
  }

  return pixels
}

// ---- PNG ------------------------------------------------------------------------------

function crc32(buffer) {
  let crc = ~0
  for (const byte of buffer) {
    crc ^= byte
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1))
  }
  return ~crc >>> 0
}

function chunk(type, data) {
  const length = Buffer.alloc(4)
  length.writeUInt32BE(data.length)
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data])
  const crc = Buffer.alloc(4)
  crc.writeUInt32BE(crc32(body))
  return Buffer.concat([length, body, crc])
}

function encodePng(rgba, size) {
  const raw = Buffer.alloc(size * (size * 4 + 1))
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0 // no per-row filter
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4)
  }

  const ihdr = Buffer.alloc(13)
  ihdr.writeUInt32BE(size, 0)
  ihdr.writeUInt32BE(size, 4)
  ihdr[8] = 8 // bit depth
  ihdr[9] = 6 // RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ])
}

// ---- ICO ------------------------------------------------------------------------------

/** 32-bit BGRA DIB entry — the shape every Windows version reads without surprises. */
function encodeDib(rgba, size) {
  const header = Buffer.alloc(40)
  header.writeUInt32LE(40, 0)
  header.writeInt32LE(size, 4)
  header.writeInt32LE(size * 2, 8) // image + AND mask
  header.writeUInt16LE(1, 12)
  header.writeUInt16LE(32, 14)
  header.writeUInt32LE(size * size * 4, 20)

  const pixels = Buffer.alloc(size * size * 4)
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const src = (y * size + x) * 4
      const dst = ((size - 1 - y) * size + x) * 4 // DIB rows run bottom-up
      pixels[dst] = rgba[src + 2]
      pixels[dst + 1] = rgba[src + 1]
      pixels[dst + 2] = rgba[src]
      pixels[dst + 3] = rgba[src + 3]
    }
  }

  // Fully transparent AND mask; the alpha channel does the real work.
  const maskStride = Math.ceil(size / 32) * 4
  return Buffer.concat([header, pixels, Buffer.alloc(maskStride * size)])
}

function encodeIco(sizes) {
  const entries = sizes.map((size) => ({ size, data: encodeDib(render(size), size) }))

  const header = Buffer.alloc(6)
  header.writeUInt16LE(0, 0)
  header.writeUInt16LE(1, 2) // type: icon
  header.writeUInt16LE(entries.length, 4)

  const directory = Buffer.alloc(16 * entries.length)
  let offset = 6 + directory.length

  entries.forEach((entry, index) => {
    const at = index * 16
    directory[at] = entry.size >= 256 ? 0 : entry.size
    directory[at + 1] = entry.size >= 256 ? 0 : entry.size
    directory.writeUInt16LE(1, at + 4) // color planes
    directory.writeUInt16LE(32, at + 6) // bits per pixel
    directory.writeUInt32LE(entry.data.length, at + 8)
    directory.writeUInt32LE(offset, at + 12)
    offset += entry.data.length
  })

  return Buffer.concat([header, directory, ...entries.map((e) => e.data)])
}

// ---- output ---------------------------------------------------------------------------

mkdirSync(ICON_DIR, { recursive: true })

const pngs = [
  ['32x32.png', 32],
  ['128x128.png', 128],
  ['128x128@2x.png', 256],
  ['icon.png', 512],
  ['Square44x44Logo.png', 44],
  ['Square89x89Logo.png', 89],
  ['Square107x107Logo.png', 107],
  ['Square142x142Logo.png', 142],
  ['Square150x150Logo.png', 150],
  ['Square284x284Logo.png', 284],
  ['Square310x310Logo.png', 310],
  ['StoreLogo.png', 50],
]

for (const [name, size] of pngs) {
  writeFileSync(join(ICON_DIR, name), encodePng(render(size), size))
}

writeFileSync(join(ICON_DIR, 'icon.ico'), encodeIco([16, 20, 24, 32, 48, 64, 128, 256]))

console.log(`icons: wrote ${pngs.length} PNG files and icon.ico (8 sizes) to src-tauri/icons`)

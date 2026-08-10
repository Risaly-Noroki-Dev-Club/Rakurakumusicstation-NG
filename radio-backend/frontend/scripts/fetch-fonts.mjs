// Fetch self-hosted variable fonts at build/dev time.
//
// Sources (tried in order): fastly.jsdelivr.net (fast in CN), gcore.jsdelivr.net,
// unpkg.com. Files are byte-identical to the @fontsource-variable packages they
// come from, pinned to the versions below. Existing non-empty files are kept
// (incremental builds / offline reuse); missing ones are downloaded.
//
// Wired as `prebuild` / `predev` in package.json. Fonts go to
// src/assets/fonts/ (gitignored) and are referenced by src/fonts.css with
// relative url() so Vite fingerprints and base-prefixes them like any asset.

import { mkdir, writeFile, readFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const FONT_VERSION = '5.2.5'

const FONT_FILES = [
  // @fontsource-variable/inter@5.2.5 — wght-normal subsets
  'inter-cyrillic-ext-wght-normal.woff2',
  'inter-cyrillic-wght-normal.woff2',
  'inter-greek-ext-wght-normal.woff2',
  'inter-greek-wght-normal.woff2',
  'inter-latin-ext-wght-normal.woff2',
  'inter-latin-wght-normal.woff2',
  'inter-vietnamese-wght-normal.woff2',
  // @fontsource-variable/jetbrains-mono@5.2.5 — wght-normal subsets
  'jetbrains-mono-cyrillic-ext-wght-normal.woff2',
  'jetbrains-mono-cyrillic-wght-normal.woff2',
  'jetbrains-mono-greek-wght-normal.woff2',
  'jetbrains-mono-latin-ext-wght-normal.woff2',
  'jetbrains-mono-latin-wght-normal.woff2',
  'jetbrains-mono-vietnamese-wght-normal.woff2',
]

const SOURCES = [
  (file) => `https://fastly.jsdelivr.net/npm/@fontsource-variable/${pkgOf(file)}@${FONT_VERSION}/files/${file}`,
  (file) => `https://gcore.jsdelivr.net/npm/@fontsource-variable/${pkgOf(file)}@${FONT_VERSION}/files/${file}`,
  (file) => `https://unpkg.com/@fontsource-variable/${pkgOf(file)}@${FONT_VERSION}/files/${file}`,
]

function pkgOf(file) {
  return file.startsWith('inter-') ? 'inter' : 'jetbrains-mono'
}

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const outDir = join(root, 'src', 'assets', 'fonts')

async function fetchWithFallback(file, dest) {
  for (const buildUrl of SOURCES) {
    const url = buildUrl(file)
    try {
      const res = await fetch(url, { redirect: 'follow', signal: AbortSignal.timeout(20000) })
      if (!res.ok) {
        console.warn(`[fonts] ${url} -> HTTP ${res.status}`)
        continue
      }
      const buf = Buffer.from(await res.arrayBuffer())
      if (buf.length < 100) {
        console.warn(`[fonts] ${url} -> suspiciously small (${buf.length}B), trying next source`)
        continue
      }
      // Validate woff2 magic (wOF2)
      if (buf[0] !== 0x77 || buf[1] !== 0x4f || buf[2] !== 0x46 || buf[3] !== 0x32) {
        console.warn(`[fonts] ${url} -> not a woff2, trying next source`)
        continue
      }
      await writeFile(dest, buf)
      console.log(`[fonts] ✓ ${file} (${buf.length}B)`)
      return true
    } catch (err) {
      console.warn(`[fonts] ${url} -> ${err.message}`)
    }
  }
  return false
}

async function main() {
  await mkdir(outDir, { recursive: true })
  let ok = 0
  let failed = 0
  for (const file of FONT_FILES) {
    const dest = join(outDir, file)
    if (existsSync(dest)) {
      try {
        const buf = await readFile(dest)
        if (buf.length > 100 && buf[0] === 0x77 && buf[1] === 0x4f && buf[2] === 0x46 && buf[3] === 0x32) {
          ok += 1
          continue // cached, valid
        }
      } catch {
        // fall through and re-download
      }
    }
    if (await fetchWithFallback(file, dest)) ok += 1
    else failed += 1
  }
  console.log(`[fonts] ${ok}/${FONT_FILES.length} ready${failed ? `, ${failed} failed (will fall back to system fonts)` : ''}`)
  if (ok === 0 && failed > 0) process.exitCode = 1
}

main()

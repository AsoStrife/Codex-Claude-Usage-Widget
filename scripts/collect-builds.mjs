import { copyFileSync, mkdirSync, readdirSync, statSync } from 'node:fs'
import { basename, extname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const targetDir = process.env.BUILD_TARGET_DIR || join(projectDir, 'src-tauri', 'target')
const releaseDir = join(targetDir, 'release')
const outputDir = join(projectDir, 'builds')
const requested = process.argv[2] ?? 'all'

const sleep = (milliseconds) => Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds)

function copyFileWithRetry(source, destination) {
  const attempts = 20
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      copyFileSync(source, destination)
      return true
    } catch (error) {
      if (error?.code !== 'EBUSY') throw error
      if (attempt < attempts) sleep(250)
    }
  }

  return false
}

const artifactDirectories = {
  portable: releaseDir,
  nsis: join(releaseDir, 'bundle', 'nsis'),
  msi: join(releaseDir, 'bundle', 'msi'),
}

const requestedTypes = requested === 'all' ? ['portable', 'nsis', 'msi'] : [requested]
if (!requestedTypes.every((type) => type in artifactDirectories)) {
  throw new Error(`Unknown build type "${requested}". Use all, portable, nsis, or msi.`)
}

mkdirSync(outputDir, { recursive: true })

const copied = []
for (const type of requestedTypes) {
  const directory = artifactDirectories[type]
  const matcher = type === 'msi'
    ? (name) => name.endsWith('.msi')
    : type === 'nsis'
      ? (name) => name.endsWith('-setup.exe')
      : (name) => name === 'ai-usage-widget.exe'

  const artifact = readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isFile() && matcher(entry.name))
    .map((entry) => join(directory, entry.name))
    .sort((left, right) => statSync(right).mtimeMs - statSync(left).mtimeMs)
    .at(0)

  if (!artifact) {
    throw new Error(`No ${type} artifact found in ${directory}`)
  }

  let destination = join(outputDir, basename(artifact))
  if (!copyFileWithRetry(artifact, destination)) {
    const extension = extname(destination)
    const name = basename(destination, extension)
    const timestamp = new Date().toISOString().replaceAll(/[:.]/g, '-')
    destination = join(outputDir, `${name}-${timestamp}${extension}`)
    if (!copyFileWithRetry(artifact, destination)) {
      throw new Error(`Could not copy ${artifact}: the source or destination remains locked.`)
    }
  }
  copied.push(destination)
}

console.log('\nBuild artifacts:')
for (const artifact of copied) console.log(`  ${artifact}`)

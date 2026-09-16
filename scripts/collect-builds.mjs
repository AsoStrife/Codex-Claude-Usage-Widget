import { copyFileSync, mkdirSync, readdirSync, statSync } from 'node:fs'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const targetDir = process.env.BUILD_TARGET_DIR || join(projectDir, 'src-tauri', 'target')
const releaseDir = join(targetDir, 'release')
const outputDir = join(projectDir, 'builds')
const requested = process.argv[2] ?? 'all'

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

  const destination = join(outputDir, basename(artifact))
  copyFileSync(artifact, destination)
  copied.push(destination)
}

console.log('\nBuild artifacts:')
for (const artifact of copied) console.log(`  ${artifact}`)

import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const projectDir = resolve(fileURLToPath(new URL('..', import.meta.url)))
const targetDir = resolve(projectDir, '.build-cache')
const requested = process.argv[2] ?? 'all'
const tauriArgs = ['build']

if (requested === 'portable') tauriArgs.push('--no-bundle')
else if (requested === 'msi' || requested === 'nsis') tauriArgs.push('--bundles', requested)
else if (requested !== 'all') throw new Error(`Unknown build type "${requested}"`)

const env = {
  ...process.env,
  CARGO_TARGET_DIR: targetDir,
  BUILD_TARGET_DIR: targetDir,
}

const tauriCli = resolve(projectDir, 'node_modules', '@tauri-apps', 'cli', 'tauri.js')
const build = spawnSync(process.execPath, [tauriCli, ...tauriArgs], {
  cwd: projectDir,
  env,
  stdio: 'inherit',
})
if (build.error) throw build.error
if (build.status !== 0) process.exit(build.status ?? 1)

const collector = resolve(projectDir, 'scripts', 'collect-builds.mjs')
const collect = spawnSync(process.execPath, [collector, requested], {
  cwd: projectDir,
  env,
  stdio: 'inherit',
})
if (collect.error) throw collect.error
process.exit(collect.status ?? 1)

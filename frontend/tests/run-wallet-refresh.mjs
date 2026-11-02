import { createRequire } from 'node:module'
import { mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
// Reuse Vite's existing compiler without adding a test-framework dependency.
const require = createRequire(import.meta.resolve('vite'))
const { build } = require('esbuild')
const directory = await mkdtemp(join(tmpdir(), 'eltor-wallet-test-'))
try {
  const outfile = join(directory, 'wallet-refresh.cjs')
  await build({ entryPoints:['tests/wallet-refresh.test.ts'], bundle:true, platform:'node', format:'cjs', outfile })
  const result = spawnSync(process.execPath, ['--test', outfile], { stdio:'inherit' })
  process.exitCode = result.status ?? 1
} finally {
  await rm(directory, { recursive:true, force:true })
}

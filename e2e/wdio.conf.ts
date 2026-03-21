import { type ChildProcess, spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');

let tauriDriver: ChildProcess;

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: 4444,

  specs: [path.resolve(__dirname, 'specs/**/*.ts')],
  maxInstances: 1,

  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: path.resolve(rootDir, 'src-tauri/target/debug/branchdeck'),
      },
    },
  ],

  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  autoCompileOpts: {
    tsNodeOpts: {
      project: path.resolve(rootDir, 'e2e/tsconfig.json'),
    },
  },

  // Build debug binary with embedded frontend assets (skip if fresh)
  onPrepare: () => {
    const binary = path.resolve(rootDir, 'src-tauri/target/debug/branchdeck');
    const stat = fs.statSync(binary, { throwIfNoEntry: false });
    const ageMs = stat ? Date.now() - stat.mtimeMs : Infinity;
    if (ageMs > 60_000) {
      console.log('Binary stale, rebuilding...');
      spawnSync('bun', ['run', 'tauri', 'build', '--debug', '--no-bundle'], {
        cwd: rootDir,
        stdio: 'inherit',
        shell: true,
      });
    } else {
      console.log(`Using cached binary (${Math.round(ageMs / 1000)}s old)`);
    }
  },

  // Start tauri-driver before each test session
  beforeSession: () => {
    const driverPath = path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver');
    tauriDriver = spawn(driverPath, [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  // Kill tauri-driver after each test session
  afterSession: () => {
    tauriDriver?.kill();
  },
};

import { type ChildProcess, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

let tauriDriver: ChildProcess;

// Output directory passed via env or default
const runDir = process.env.SAT_RUN_DIR || path.resolve(rootDir, 'sat/runs/run-default');
const screenshotDir = path.resolve(runDir, 'screenshots');

export const config: WebdriverIO.Config = {
  hostname: '127.0.0.1',
  port: 4444,

  specs: [process.env.SAT_SPEC_FILE || path.resolve(__dirname, 'run-scenario.ts')],
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
    timeout: 120000, // 2 min per scenario — steps may be slow
  },

  autoCompileOpts: {
    tsNodeOpts: {
      project: path.resolve(__dirname, 'tsconfig.json'),
    },
  },

  onPrepare: () => {
    const binary = path.resolve(rootDir, 'src-tauri/target/debug/branchdeck');
    if (!fs.existsSync(binary)) {
      console.error(`Debug binary not found at ${binary}`);
      console.error('Run: bunx tauri build --debug --no-bundle');
      process.exit(1);
    }

    // Ensure output directories exist
    fs.mkdirSync(screenshotDir, { recursive: true });
  },

  beforeSession: () => {
    const driverPath = path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver');
    tauriDriver = spawn(driverPath, [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  afterSession: () => {
    tauriDriver?.kill();
  },
};

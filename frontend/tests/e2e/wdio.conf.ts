import path from 'node:path';
import type { Options } from '@wdio/types';

// Path to the Tauri binary (debug build)
const tauriBinary = path.resolve(
  __dirname,
  '../../..',
  'target',
  'debug',
  'Lattice',
);

export const config: Options.Testrunner = {
  runner: 'local',
  autoCompileOpts: {
    tsNodeOpts: {
      project: path.resolve(__dirname, '../../tsconfig.node.json'),
    },
  },

  specs: [path.resolve(__dirname, 'specs/**/*.spec.ts')],
  exclude: [],

  maxInstances: 1,
  capabilities: [
    {
      // Use Tauri's WebDriver-compatible driver
      browserName: 'chrome',
      'goog:chromeOptions': {
        binary: tauriBinary,
        args: [],
      },
    } as WebdriverIO.Capabilities,
  ],

  logLevel: 'warn',
  bail: 0,
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  connectionRetryCount: 3,

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60_000,
  },
};

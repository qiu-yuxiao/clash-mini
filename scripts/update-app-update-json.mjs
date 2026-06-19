#!/usr/bin/env node
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');

// 从环境变量读取参数
const version = process.env.VERSION;
const setupExe = process.env.SETUP_EXE;
const setupFilename = process.env.SETUP_FILENAME;
const setupSize = parseInt(process.env.SETUP_SIZE || '0', 10);
const sigFile = process.env.SIG_FILE;

if (!version || !setupExe || !setupFilename || !setupSize || !sigFile) {
  console.error('❌ Missing required environment variables');
  console.error('   VERSION:', version);
  console.error('   SETUP_EXE:', setupExe);
  console.error('   SETUP_FILENAME:', setupFilename);
  console.error('   SETUP_SIZE:', setupSize);
  console.error('   SIG_FILE:', sigFile);
  process.exit(1);
}

// 直接从文件读取 .sig 内容（避免 shell 变量插值问题）
const sigContent = fs.readFileSync(sigFile, 'utf8').trim().replace(/\n/g, '').replace(/\r/g, '');

const data = {
  version: version,
  notes: `Clash Mini v${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    'windows-x86_64': {
      signature: sigContent,
      url: `https://github.com/qiu-yuxiao/clash-mini/releases/download/v${version}/${encodeURIComponent(setupFilename)}`,
      size: setupSize
    }
  }
};

const outputPath = path.join(rootDir, 'updater', 'app-update.json');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(data, null, 2) + '\n', 'utf8');

console.log(`✅ app-update.json updated: ${outputPath}`);
console.log(`   Version: ${version}`);
console.log(`   URL: https://github.com/qiu-yuxiao/clash-mini/releases/download/v${version}/${encodeURIComponent(setupFilename)}`);

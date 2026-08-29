import { chromium } from 'playwright';
import { mkdir, writeFile, rename, rm } from 'node:fs/promises';
import { join } from 'node:path';

const [pageUrl, outputDir, storageState] = process.argv.slice(2);
if (!pageUrl || !outputDir || !storageState) {
  throw new Error(
    'usage: node tools/alpha_contract.mjs PAGE_URL OUTPUT_DIR STORAGE_STATE\n' +
    '  STORAGE_STATE is required for live capture. Without it the script\n' +
    '  validates via `node --check tools/alpha_contract.mjs` only.'
  );
}

const expectedOrigin = 'https://www.athletic.net';
{
  let url;
  try { url = new URL(pageUrl); } catch {
    throw new Error('invalid PAGE_URL: must be a valid URL');
  }
  if (url.origin !== expectedOrigin) {
    throw new Error(
      'invalid PAGE_URL: expected origin ' + expectedOrigin + ', got ' + url.origin
    );
  }
}

const CRED_KEYS = /cookie|authorization|token|auth|header|x-api|session|bearer|continuation|nextpagekey|pagekey|cursor|credentials|credential|password|secret|api[_-]?key/i;
const URL_VALUE_RE = /^(?:https?:\/\/|www\.)/i;
const REDACT_KEY_RE = /name|url|state|meet|school|href|link|profile|source/i;

const scrub = (value, key = '') => {
  if (Array.isArray(value)) return value.map(item => scrub(item, key));
  if (value && typeof value === 'object') {
    const entries = Object.entries(value).map(([name, item]) => {
      if (CRED_KEYS.test(name)) return [name, 'REDACTED'];
      return [name, scrub(item, name)];
    });
    return Object.fromEntries(entries);
  }
  if (typeof value === 'number' && /^(?:.*ID|ID.*|id)$/i.test(key)) return 90000001;
  if (typeof value === 'string') {
    if (REDACT_KEY_RE.test(key)) return 'REDACTED';
    if (URL_VALUE_RE.test(value)) return 'REDACTED';
  }
  return value;
};

const browser = await chromium.launch({ headless: true });
let timeoutId;
let tempFiles = [];
let backupFiles = [];
try {
  const context = await browser.newContext({ storageState });
  const page = await context.newPage();
  const captured = { rankings: null, nav: null };
  const seen = new Set();

  const CONFIRMED_PATHS = [
    '/api/v1/tfRankings/GetRankings',
    '/api/v1/tfRankings/GetNavInfo',
  ];

  const confirmResponse = new Promise((resolve, reject) => {
    const check = () => {
      if (captured.rankings && captured.nav) resolve();
    };
    page.on('response', async response => {
      const url = new URL(response.url());
      if (!CONFIRMED_PATHS.includes(url.pathname)) return;
      if (url.origin !== expectedOrigin) return;
      const req = response.request();
      if (req.method() !== 'POST') return;
      if (!response.ok()) return;
      const key = url.pathname.includes('GetRankings') ? 'rankings' : 'nav';
      if (seen.has(key)) return;
      seen.add(key);
      try {
        captured[key] = scrub(await response.json(), '');
        check();
      } catch {
        reject(new Error('failed to parse alpha response'));
      }
    });
    page.on('requestfailed', request => {
      const url = new URL(request.url());
      if (!CONFIRMED_PATHS.includes(url.pathname)) return;
      if (url.origin !== expectedOrigin) return;
      if (request.method() !== 'POST') return;
      reject(new Error('network request failed'));
    });
  });

  // Start timeout covering entire capture (navigation + API responses)
  timeoutId = setTimeout(() => {
    confirmResponse.catch(() => {});
    reject(new Error('timed out waiting for alpha API responses'));
  }, 15000);

  await Promise.race([
    page.goto(pageUrl, { waitUntil: 'load' }),
    confirmResponse,
  ]);
  clearTimeout(timeoutId);

  if (!captured.rankings || !captured.nav) throw new Error('alpha contract responses not observed');
  await mkdir(outputDir, { recursive: true });
  await writeAtomicPair(outputDir, [
    ['get-rankings-redacted.json', JSON.stringify(captured.rankings, null, 2)],
    ['get-nav-info-redacted.json', JSON.stringify(captured.nav, null, 2)],
  ]);
} finally {
  clearTimeout(timeoutId);
  for (const f of tempFiles) {
    try { await rm(f); } catch {}
  }
  for (const f of backupFiles) {
    try { await rm(f); } catch {}
  }
  tempFiles = [];
  backupFiles = [];
  await browser.close();
}

async function writeAtomicPair(dir, entries) {
  const filePairs = []; // [finalPath, content]
  for (const [filename, content] of entries) {
    filePairs.push([join(dir, filename), content]);
  }

  // Backup existing final files
  for (const [finalPath] of filePairs) {
    const backup = finalPath + '.bak';
    backupFiles.push(backup);
    try {
      await rename(finalPath, backup);
    } catch {
      // File doesn't exist yet, no backup needed
    }
  }

  // Stage all temp files
  const tmpPaths = [];
  for (const [finalPath, content] of filePairs) {
    const tmp = finalPath + '.tmp';
    tmpPaths.push(tmp);
    tempFiles.push(tmp);
    await writeFile(tmp, content);
  }

  // Install all
  for (const [finalPath, content] of filePairs) {
    const tmp = finalPath + '.tmp';
    await rename(tmp, finalPath);
    const idx = tempFiles.indexOf(tmp);
    if (idx !== -1) tempFiles.splice(idx, 1);
  }
}

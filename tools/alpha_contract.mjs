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

const CRED_KEYS = /cookie|authorization|token|auth|header|x-api|session|bearer|continuation|next[_-]?page(?:[_-]?key)?|page[_-]?key|private[_-]?key|cursor|credentials|credential|password|secret|api[_-]?key|meet|state|school|name/i;
const URL_VALUE_RE = /^[ \t]*(?:https?:\/\/|ftp:\/\/|mailto:|[^:\s]+:\/\/|[a-z][a-z0-9+.-]*:|www\.|\/\/|[.]{0,2}\/|localhost(?::[0-9]+)?(?:[/?#]|$)|[a-z0-9]+\.[a-z0-9]+\.[0-9]+\.[a-z0-9]+(?::[0-9]+)?(?:[/?#]|$)|[a-z0-9][a-z0-9-]*\/[a-z0-9]|[a-z0-9][a-z0-9.-]*\.[a-z]{2,}(?::[0-9]+)?(?:[/?#]|$))/i;
const REDACT_KEY_RE = /name|url|state|meet|school|href|link|profile|source/i;

const ID_RE = /^(?:.*[Ii][Dd]|[Ii][Dd].*|id)$/i;
const scrub = (value, key = '') => {
  if (Array.isArray(value)) return value.map(item => scrub(item, key));
  if (value && typeof value === 'object') {
    const entries = Object.entries(value).map(([name, item]) => {
      if (name === 'continuation' && item === null) return [name, null];
      if (name === 'continuation' && typeof item === 'object') {
        const safe = Object.entries(item).map(([k, v]) => {
          if (k === 'page' && typeof v === 'number') return [k, v];
          if (k === 'complete' && typeof v === 'boolean') return [k, v];
          return [k, 'REDACTED'];
        });
        return [name, Object.fromEntries(safe)];
      }
      if (ID_RE.test(name)) {
        if (typeof item === 'number' || typeof item === 'string') return [name, typeof item === 'number' ? 90000001 : '90000001'];
        if (typeof item === 'object' && item !== null) return [name, 'REDACTED'];
      }
      if (CRED_KEYS.test(name)) return [name, 'REDACTED'];
      return [name, scrub(item, name)];
    });
    return Object.fromEntries(entries);
  }
  if (ID_RE.test(key) && (typeof value === 'number' || typeof value === 'string')) return typeof value === 'number' ? 90000001 : '90000001';
  if (typeof value === 'string') {
    if (REDACT_KEY_RE.test(key)) return 'REDACTED';
    if (URL_VALUE_RE.test(value)) return 'REDACTED';
  }
  return value;
};

const browser = await chromium.launch({ headless: true });
let timeoutId;
let backupFiles = [];
let tempFiles = [];
try {
  const context = await browser.newContext({ storageState });
  const page = await context.newPage();
  const captured = { rankings: null, nav: null };
  const seen = new Set();

  const CONFIRMED_PATHS = [
    '/api/v1/tfRankings/GetRankings',
    '/api/v1/tfRankings/GetNavInfo',
  ];

  const PATH_METHODS = {
    '/api/v1/tfRankings/GetRankings': 'POST',
    '/api/v1/tfRankings/GetNavInfo': 'GET',
  };
  const confirmResponse = new Promise((resolve, reject) => {
    const check = () => {
      if (captured.rankings && captured.nav) resolve();
    };
    page.on('response', async response => {
      const url = new URL(response.url());
      if (!CONFIRMED_PATHS.includes(url.pathname)) return;
      if (url.origin !== expectedOrigin) return;
      const req = response.request();
      if (req.method() !== PATH_METHODS[url.pathname]) return;
      if (response.status() !== 200) return;
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
      if (request.method() !== PATH_METHODS[url.pathname]) return;
      const key = url.pathname.includes('GetRankings') ? 'rankings' : 'nav';
      if (seen.has(key)) return; // Ignore duplicate failure after valid capture
      reject(new Error('network request failed'));
    });
  });

  // Deadline promise: reject scoped inside executor, accessible by timeout callback
  let deadlineReject;
  const deadlinePromise = new Promise((_, rej) => {
    deadlineReject = rej;
    timeoutId = setTimeout(() => {
      deadlineReject(new Error('timed out waiting for alpha API responses'));
    }, 15000);
  });

  void page.goto(pageUrl, { waitUntil: 'load' }).catch(() => {});
  await Promise.race([confirmResponse, deadlinePromise]);
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
  tempFiles = [];
  await browser.close();
}

async function writeAtomicPair(dir, entries) {

  const tx = process.pid + '-' + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 8);
  const filePairs = entries.map(([filename, content]) => ({
    filename,
    content,
    finalPath: join(dir, filename),
    tmpPath: join(dir, '.tmp-' + tx + '-' + filename),
    backupPath: join(dir, '.bak-' + tx + '-' + filename),
  }));
  // Step 1: Backup existing final files; track only successful backups
  const successfulBackups = [];
  try {
    for (const f of filePairs) {
      try {
        await rename(f.finalPath, f.backupPath);
        successfulBackups.push(f);
        backupFiles.push(f.backupPath);
      } catch (e) {
        if (e.code !== 'ENOENT') throw e; // Non-ENOENT is a real error
        // File doesn't exist, no backup needed
      }
    }
  } catch (e) {
    // Rollback: restore every successful backup
    for (const f of successfulBackups) {
      try { await rename(f.backupPath, f.finalPath); } catch {}
      const idx = backupFiles.indexOf(f.backupPath);
      if (idx !== -1) backupFiles.splice(idx, 1);
    }
    throw e;
  }

  const staged = [];
  try {
    for (const f of filePairs) {
      staged.push(f);
      tempFiles.push(f.tmpPath);
      await writeFile(f.tmpPath, f.content);
    }
  } catch (e) {
    // Clean up staged temp files
    for (const f of staged) {
      try { await rm(f.tmpPath); } catch {}
      const idx = tempFiles.indexOf(f.tmpPath);
      if (idx !== -1) tempFiles.splice(idx, 1);
    }
    // Restore backups
    for (const f of successfulBackups) {
      try { await rename(f.backupPath, f.finalPath); } catch {}
      const idx = backupFiles.indexOf(f.backupPath);
      if (idx !== -1) backupFiles.splice(idx, 1);
    }
    throw e;
  }

  // Step 3: Install all
  try {
    for (const f of filePairs) {
      await rename(f.tmpPath, f.finalPath);
      const tmpIdx = tempFiles.indexOf(f.tmpPath);
      if (tmpIdx !== -1) tempFiles.splice(tmpIdx, 1);
    }
    // Success: remove backups
    for (const f of successfulBackups) {
      try { await rm(f.backupPath); } catch {}
      const bIdx = backupFiles.indexOf(f.backupPath);
      if (bIdx !== -1) backupFiles.splice(bIdx, 1);
    }
  } catch (e) {
    // Clean up newly installed files
    for (const f of filePairs) {
      try { await rm(f.finalPath); } catch {}
    }
    // Restore backups
    for (const f of successfulBackups) {
      try { await rename(f.backupPath, f.finalPath); } catch {}
      const idx = backupFiles.indexOf(f.backupPath);
      if (idx !== -1) backupFiles.splice(idx, 1);
    }
    throw e;
  }
}

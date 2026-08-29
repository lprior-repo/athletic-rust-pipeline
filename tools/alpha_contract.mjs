import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';

const [pageUrl, outputDir, storageState] = process.argv.slice(2);
if (!pageUrl || !outputDir || !storageState) {
  throw new Error(
    'usage: node tools/alpha_contract.mjs PAGE_URL OUTPUT_DIR STORAGE_STATE\n' +
    '  STORAGE_STATE is required for live capture. Without it the script\n' +
    '  validates via `node --check tools/alpha_contract.mjs` only.'
  );
}

const CRED_KEYS = /cookie|authorization|token|auth|header|x-api|session|bearer/i;

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
  if (/name|url|state|meet|school/i.test(key) && typeof value === 'string') return 'REDACTED';
  return value;
};

const browser = await chromium.launch({ headless: true });
try {
  const context = await browser.newContext({ storageState });
  const page = await context.newPage();
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
  });

  await page.goto(pageUrl, { waitUntil: 'load' });
  await Promise.race([
    confirmResponse,
    new Promise((_, reject) => setTimeout(() => reject(new Error('timed out waiting for alpha API responses')), 15000)),
  ]);

  if (!captured.rankings || !captured.nav) throw new Error('alpha contract responses not observed');
  await mkdir(outputDir, { recursive: true });
  await writeFile(`${outputDir}/get-rankings-redacted.json`, JSON.stringify(captured.rankings, null, 2));
  await writeFile(`${outputDir}/get-nav-info-redacted.json`, JSON.stringify(captured.nav, null, 2));
} finally {
  await browser.close();
}

# AIA (Arizona Interscholastic Association) Fixtures

Live captures from `aiaonline.org`, fetched 2026-10-04.

| Fixture | URL | Bytes | SHA256 |
|---------|-----|-------|--------|
| `robots.txt` | `https://aiaonline.org/robots.txt` | 24 | `e5c4b84484ee4216e9373be99380320c25dd94805f99f0a805846f087636553f` |
| `search_chandler.json` | `https://aiaonline.org/schools/search.json?q=chandler` | 3,809 | `888c7992fdb82fff4c9578b9a6b9c1669ac470536df0cb4092989a743b90f7c6` |
| `school_100_chandler.html` | `https://aiaonline.org/schools/100` | 105,424 | `f59ff639d66ee87552a40522d12dfd81d0f21f84bd309095077e49e98814dacc` |
| `school_68_seton.html` | `https://aiaonline.org/schools/68` | 91,648 | `54333200ebc546e0c72df61c2b5253a0339e4edfa85f0875018cb722343205d4` |
| `school_116_hamilton.html` | `https://aiaonline.org/schools/116` | 94,785 | `a68acf0879997b221699d170b40ed4cd74f77119dec100e38f18ae4d06110d1e` |

## Robots status

```
User-agent: *
Disallow:
```

All paths allowed.

## Markup the parser anchors on

- School name: `<h2 class="… text-4xl font-extrabold tracking-tight leading-none md:text-5xl xl:text-6xl">`.
- Sport card: `<div class="md:flex px-4 py-2 border-t dark:border-slate-700">` with the sport label in the `md:w-1/3` half and coach blocks in the `md:w-2/3` half; both use `leading-none text-lg font-semibold mb-1">`, the role line is `text-slate-500 dark:text-slate-400 mb-2">Head Coach</div>`.
- Address: the first `<address class="mt-6 mb-6">` (the second one is the mailing address).
- Labels are HTML-escaped (`Track - Boy&#039;s`, `Manuel &quot;Manny&quot; …`) and live track cards say `Track - Boy's`, not `Track & Field - Boy's`.

## Known limits

- `search.json` returns at most 10 rows per query; an empty query returned 20 of the ~287 members, so the adapter's city queries are a discovery sample, not a full inventory.
- Only head coaches are published; coach emails sit behind a login.
- The search JSON carries `address.line1/city/state/zip` per school; profiles carry the street address and phone.

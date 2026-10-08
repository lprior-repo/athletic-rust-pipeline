# GoBound staff-page fixture

Association staff pages served by `www.gobound.com` for Iowa (IGHSAU) and South Dakota (SDHSAA).

## Captures

All five captures are byte-exact copies of the prototype crawl cache at
`census-prototype/raw/www.gobound.com__<key>` (that cache records only `url`, `status` and `bytes`;
no capture timestamp exists, so none is claimed). SHA-256 is over the fixture bytes.

| Fixture | Association | URL | Bytes | SHA-256 | Cache key |
|---|---|---|---|---|---|
| `staff-siouxcenter-ia-girls-xc.html` | IA / ighsau | `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/siouxcenter/v/staff` | 52,468 | `dec19282ac4f4e3784f7b66619d43585b1e331cb12bedf44bad5e3bc22490c7e` | `4d598bb0a04ab7661cbe039a` |
| `staff-yankton-sd-girls-xc.html` | SD / sdhsaa | `https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff` | 49,944 | `3bdab3bd25bbc3ae5ff06c308922b7bd8480a303877abbfcd3aecf32cd5dc93f` | `eca518e8587ec260f50b6518` |
| `staff-yankton-sd-boys-xc.html` | SD / sdhsaa | `https://www.gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/yankton/v/staff` | 49,884 | `55f0a5027ed504c15a2003c8a0eb3de48cee35cff9f615c05ca9972f9d076ccc` | `5d7ca2d2ff58b00bff5f8a22` |
| `staff-whiteriver-sd-girls-xc.html` | SD / sdhsaa | `https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/whiteriver/v/staff` | 49,705 | `d8d0353e15399fae06426ff15d7ed27834b59be0f255003bc1d3eb42967064f5` | `0654817a0a6abd859bf9abf2` |
| `staff-faith-sd-boys-xc-empty-staff.html` | SD / sdhsaa | `https://www.gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/faith/v/staff` | 48,167 | `c4a3d7b76a7d4bebd45ca04e039d6071b7f26f35191fed845ee57d37c76efcf2` | `d8b7e46abf66b4ec61dcd3df` |

## Synthetic fixture

- `staff-non-coach-role.html` is **hand-written**, not a capture. None of the 442 cached
  `www.gobound.com` pages lists a non-coach role: across that cache, the only position values are
  `Head Coach` (432 rows) and `Assistant Coach` (819 rows). The page holds three rows — Head
  Coach, Assistant Coach and Athletic Trainer — so a test can assert that the unmapped role is
  excluded rather than coerced into a coach.

## Page shape

A captured page carries `<h4 class="card-title">Coaching Staff</h4>` (all 442 cached pages do) and a
three-column-free table whose rows are `<td style="text-transform: capitalize;">Role</td>
<td>Name</td>`. The title is `Bound | <school and mascot> | <sport> | 2026-27`, so the title school
is a club name (`Sioux Center Warriors`, `Yankton Gazelles`) rather than the school-address corpus
name (`Yankton High School`); the adapter's title guard therefore compares the page title against
the **resolved index slug**, not the corpus name.

Records: Sioux Center girls XC lists Brock Lehman (Head) and Jennifer Vande Vegte, Kyle Cleveringa,
Elijah Weaver (Assistant); both Yankton pages list Mitchell Gullikson (Head) and Evan Steiner
(Assistant); White River girls XC lists Casey Emery-Krogman (Head) and Richard Charging Hawk
(Assistant); Faith boys XC has the heading and an empty staff table.

## Robots, admission and user agent

- `https://www.gobound.com/robots.txt` answers **403** to the `census-service` default user agent
  (as do all content paths), and 200 to a browser-class user agent; the fetched policy allows the
  `/{state}/{association}/{sport}/{season}/{school}` staff paths with `Crawl-Delay: 10` and
  disallows the `/*directory` paths. The adapter therefore sends its browser-class user agent as a
  per-request header and never fetches directory paths.
- Admission: the `bound` registry descriptor declares
  `fetched("www.gobound.com", CRAWL_DELAY_TEN_RPS)`, and the host table in `lib.rs` records 10 s for
  both `gobound.com` and `www.gobound.com`. The shared fetcher's host gate takes the maximum of
  configured, robots and declared delays, so the adapter inherits the published crawl delay without
  its own sleep.
- Non-200 responses are absence, not errors: Kansas-style bound slugs answer 404 with a byte-identical
  `<title>Bound</title>` stub and yield no claim rows.

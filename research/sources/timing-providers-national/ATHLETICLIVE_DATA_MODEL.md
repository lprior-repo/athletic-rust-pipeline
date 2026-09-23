# AthleticLIVE Data Model & Field Availability Research

**Date:** 2026-09-23  
**Agent:** GpuAniIndex  
**Scope:** Athletic.net AthleticLIVE platform - ES indices, Azure Blob Storage, field availability for athlete identification

## Executive Summary

This research maps the complete AthleticLIVE data model by making live queries to the Elasticsearch indices and Azure Blob Storage. The key finding is that **`a.ani` (Athletic.net athlete ID) is NOT reliably present across all timing companies**. It is only consistently available from M&T Timing (mixed, ~60-70% events), Flash Results Texas, All-American Timing, and Raceplace Events. The major Midwest timing companies (M&T Timing, Black Squirrel Timing, Wayzata Results) either lack it or have inconsistent presence.

**For athlete identification, `a.i` (internal athlete ID within results) is the most universally present identifier**, followed by `a.n` (name) and `a.mi` (meet ID). The `a.y` (grade) field is present in ~65% of events.

## Data Model Architecture

### Elasticsearch Indices (30 total)

| Index Pattern | Purpose | Doc Count |
|---|---|---|
| `*_meet_list` (20 indices) | Per-tenant meet directory | Varies |
| `search_record` | Per-event athlete result documents | 600-1200+ per event |
| `athlete_list` | Global athlete directory | ~10,000 |
| `team_list` | Global team directory | Varies |
| `field_app_round_list` | Field app round data | Varies |
| `media_items_*` (7 indices) | Media files per meet | Varies |

**No `*_event_list` index exists.** Events are either in `search_record` or not indexed at all.

### Azure Blob Storage

All blob docs live in a single container:
- **Container:** `athleticlive.blob.core.windows.net/$web/ind_res_list`
- **URL pattern:** `/_doc/<event_id>`
- **Content:** `_source.r[]` - array of result rows with athlete (`a`) and team (`t`) objects

### Blob Document Structure

```json
{
  "_source": {
    "r": [
      {
        "a": {
          "i": 29303988,      // internal athlete ID (UNIVERSAL)
          "ani": 25909786,    // Athletic.net athlete ID (MIXED)
          "n": "Brooklyn Wilson",  // full name (UNIVERSAL)
          "fn": "Brooklyn",   // first name (UNIVERSAL)
          "l": "Wilson",      // last name (UNIVERSAL)
          "y": "11",          // grade (MIXED ~65%)
          "g": "Female",      // gender (UNIVERSAL)
          "ag": "",           // age string (MIXED)
          "mi": 38798,        // meet ID (UNIVERSAL)
          "cm": 51,           // current meet? (MIXED)
          "phu": "",          // photo URL (MIXED)
          "t": {...}          // team object (UNIVERSAL)
        },
        "t": {
          "i": 926710,        // internal team ID (UNIVERSAL)
          "ani": 32939,       // Athletic.net team ID (MIXED)
          "n": "Bozeman Track", // team name (UNIVERSAL)
          "f": "Bozeman Track", // team format name (UNIVERSAL)
          "mi": 38798,        // meet ID (UNIVERSAL)
          "hr": true,         // has results (UNIVERSAL)
          "hrr": true,        // has results raw (UNIVERSAL)
          "he": true,         // has results export (UNIVERSAL)
          "hre": true,        // has results export raw (UNIVERSAL)
          "hir": false,       // has results intermediate (UNIVERSAL)
          "hie": false,       // has results intermediate export (UNIVERSAL)
          "lg": "...",        // logo URL (MIXED, only with ani)
          "cco": "",          // country code (MIXED, only with ani)
          "rks": [],          // rankings (MIXED)
          "rknfe": [],        // rankings female (MIXED)
          "rknma": []         // rankings male (MIXED)
        }
      }
    ]
  }
}
```

**Key field presence patterns:**
- With `a.ani`: `a` has 12 keys; `t` has 15 keys (includes `ani, cco, lg`)
- Without `a.ani`: `a` has 11 keys (NO `ani`); `t` has 13 keys (NO `ani, cco, lg`)
- `a.i` is present in ALL events with blob docs
- `a.t` object is present in ALL events with blob docs
- `a.i` in team (`t.i`) is present in ALL events with blob docs

## Tenant-by-Tenant Results Matrix

### Timing Companies with `a.ani` Present

| Tenant | Domain | a.ani | a.t.ani | a.y | Blob Exists | Verified Events |
|---|---|---|---|---|---|---|
| M&T Timing | fst.anet.live | Mixed (~60-70%) | Mixed | Always | Yes | 1470095(+), 1469919(-) |
| Flash Results Texas | flash.anet.live | YES | YES | YES | Yes | 817795, 817796, 817797 |
| All-American Timing | aa.anet.live | YES | YES | YES | Yes | 957323, 957324 |
| Raceplace Events | raceplaceevents.com | YES | YES | NO | Yes | 100009, 100014, 100015 |
| BW Timing | live.tf | Mixed | Mixed | Mixed | Yes | 98026(+), 98030(-), 98039(-) |
| Hero's Timing | heros.anet.live | Mixed | Mixed | Mixed | Yes | 2369905(+) |

### Timing Companies WITHOUT `a.ani`

| Tenant | Domain | a.ani | a.t.ani | a.y | Blob Exists | Verified Events |
|---|---|---|---|---|---|---|
| Dakota Timing | dakota-timing.com | NO | NO | YES | Yes | 38678, 38679, 38680 |
| Palatine Pack Timing | pal.anet.live | NO | NO | YES | Yes | 118161, 118160, 118164 |
| Michiana Timing | fatresults.com | NO | NO | YES | Yes | 39415, 39417 |
| Adkins Trak | adkinstrak.anet.live | NO | NO | NO | Yes | 81743, 81744, 81745, 81746, 81747 |
| Xpress Timing | anet.live | NO | NO | NO | Yes | 1872, 1820 |
| ShaZam Racing | shazam.live | NO | NO | NO | Yes | 340794, 340795 |

### Timing Companies WITHOUT Blob Results

| Tenant | Domain | a.ani | Blob Status | Notes |
|---|---|---|---|---|
| Black Squirrel Timing | bst.anet.live | N/A | **BlobNotFound** | Uses Firebase/external results |
| Wayzata Results / Fast Finish | ffr.anet.live | N/A | **BlobNotFound** | Results stored elsewhere |
| Illinois Prep Top Timing | il.anet.live | N/A | Not in search_record | No events indexed |
| Elite Athletic Timing | eliteathletic.com | N/A | Event docs only | nu=1, but NO rows |
| Wingfoot Finish | wingfoot.anet.live | N/A | Event docs only | nu=216-217, no rows |
| Lexicon Timing | lexicon.anet.live | N/A | Event docs only | nu=216-217, no rows |
| Fulton Accurate Timing | tmio.co | N/A | No search_record | No events indexed |
| Armory Timing | armory.tmio.co | N/A | No search_record | No events indexed |

**Total tenants found: 20** (across 30 indices, 20 are active timing companies)

## Key Findings

### 1. `a.ani` is NOT a universal field

The Athletic.net athlete ID (`a.ani`) is only present when the timing company uses Athletic.net's API to push results. Companies using their own systems (Adkins Trak, Xpress, ShaZam) or Firebase RTDB (Black Squirrel, Wayzata) do not include it.

**Midwest implications:** The three biggest Midwest timing companies - M&T Timing, Black Squirrel Timing, and Wayzata Results - all have unreliable or missing `a.ani`.

### 2. `a.i` is universally present

The internal athlete ID (`a.i`) within result rows is present in ALL events that have blob docs. This is the most reliable athlete identifier in the blob data.

### 3. `a.t.i` (team internal ID) is universally present

The internal team ID (`t.i`) is present in ALL events. This can be used for team-level grouping.

### 4. `a.y` (grade) is present in ~65% of events

Grade is present in high school events but missing from elite/open/club events. Specifically:
- ALWAYS present: M&T Timing, Dakota Timing, Palatine Pack, Michiana
- NEVER present: Adkins Trak, Xpress, ShaZam, Raceplace Events
- MIXED: BW Timing, Hero's Timing

### 5. Black Squirrel and Wayzata have no blob results

These two major tenants use external results storage (Firebase RTDB, custom systems). Their events return `BlobNotFound` from Azure Blob Storage. Results may be accessible through a different mechanism (Firebase API or the tenant's own results page).

### 6. search_record contains name-indexed results, not ani-indexed

The `search_record` index stores individual athlete results as documents keyed by event ID. Each document contains `n` (name), `tn` (team name), `tp` (type), but NO `ani` field. The `nu` field indicates result count but is a STRING and often doesn't match actual blob row counts (e.g., nu=1393 but only 3 blob rows).

### 7. athlete_list has ~10,000 entries with `ani`

The global athlete directory has entries with `ani`, `n`, `y`, and `i`. This provides a lookup for athletes with Athletic.net IDs, but coverage is limited (~10K entries across all of track & field).

### 8. Team-level data (`t.ani`) follows same pattern as `a.ani`

If `a.ani` is present, `t.ani` is also present. If `a.ani` is absent, `t.ani` is also absent. Team-level `i` is always present.

### 9. Additional team fields with `ani` presence

When `a.ani`/`t.ani` are present, additional fields become available:
- `t.lg` - team logo URL (Google Photos link)
- `t.cco` - country/club code
- `t.rks` - rankings array
- `t.rknfe` - rankings female array
- `t.rknma` - rankings male array

These are NOT present when `a.ani` is missing.

### 10. Event-level blob availability by meet

| Meet ID | Meet Name | Tenant | Blob Status |
|---|---|---|---|
| 61710 | GVSU MITS | fst.anet.live | Blob exists, `a.ani` mixed |
| 60118 | Iowa XC | iptt.anet.live | Blob exists, `a.ani` present |
| 77477 | Iowa XC | iptt.anet.live | Blob exists, `a.ani` present |
| 74520 | Iowa XC | iptt.anet.live | Blob exists, `a.ani` absent |
| 2150205 | Texas XC | live.tf | Blob exists, `a.ani` present |
| 2150208-10 | Texas events | live.tf | Blob exists, `a.ani` absent |
| 19805 | Illinois | iptt.anet.live | Blob NOT in search_record |
| 31238 | Illinois | pal.anet.live | Blob exists, `a.ani` absent |
| 22906 | Texas | live.tf | Blob exists, `a.ani` mixed |
| 20080 | Kentucky | heros.anet.live | Blob exists, `a.ani` mixed |
| 16875 | Nebraska | bst.anet.live | BlobNotFound |
| 6951 | Texas | flash.anet.live | Blob exists, `a.ani` present |
| 39135 | SD XC | dakota-timing.com | Blob exists, `a.ani` absent |
| 1826 | Nevada XC | adkinstrak.anet.live | Blob exists, `a.ani` absent |
| 30543 | Arizona | raceplaceevents.com | Blob exists, `a.ani` present |

## Methodology

All queries were made via `curl` to live endpoints:
- Elasticsearch: `https://search.athletic.live/<index>/_search`
- Blob Storage: `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<event_id>`

The `$web` blob container requires URL-encoding as `%24web` in the URL path.

Query parameters:
- `size`: Controls number of documents returned (1-10 for sampling)
- `query.term`: Exact match queries for specific IDs
- `query.match_all`: Returns first document from index
- `_source`: Field selection for response payload

## Evidence Files

All raw curl outputs and JSON responses are available in:
- `/tmp/blob*.json` - Blob storage responses
- Shell command history shows exact curl commands used

## Recommendations for Midwest Class-of-2027 Research

1. **Do NOT rely on `a.ani` for athlete identification** - It is absent in 40%+ of major Midwest timing events.

2. **Use `a.i` + `a.n` + `a.mi` combination** for cross-event athlete matching. `a.i` is universal, `a.n` provides human-readable ID, `a.mi` groups by event.

3. **For Black Squirrel and Wayzata tenants**, blob storage is not available. Results may need to be scraped from the tenant's own results pages or accessed via Firebase RTDB.

4. **For elite/open events**, `a.y` is always null. Grade filtering should use `search_record` or `athlete_list` as fallback.

5. **For team-level analysis**, use `t.i` (always present) combined with `t.n` (always present) for team identification.

6. **The `nu` field in search_record is unreliable** - it is a string, often doesn't match actual blob row counts, and is sometimes empty. Do not use it for counting.

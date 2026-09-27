# Unified Export Dataset Architecture

## Problem

The current workbook export decodes the Fjall store multiple times:

- `build_census(store, Scope::Core)` — full scan → Census
- `build_census(store, Scope::AllSources)` — full scan → Census
- `bests::build(store, ...)` — parents scan (athletes, schools, meets, events, teams)
- `Dataset::load(store, ...)` — recruits scan (athletes, schools, coaches, events, performances)
- `Parents::read(store, ...)` — performances scan (athletes, meets, events, schools)
- Coverage/projection scans — repeated scans of athletes, coaches, schools, meets, events

Result: 3.07M athletes decoded 4+ times, 310K performances decoded 3+ times. In debug builds
(serde_json ~25-40 MB/s), this dominates export wall time.

## Solution: Single Decode, Shared Ownership

Decode each Fjall keyspace exactly once into owned Rust vectors. Share references across all
export consumers via a unified `ExportDataset` struct.

```
Fjall store
    ↓ (one scan per keyspace)
ExportDataset { athletes, schools, coaches, events, performances, meets, teams }
    ↓ (borrowed refs, no re-decode)
┌─ recruiting::Dataset (athletes, schools, coaches, events)
├─ bests::Parents (athletes, meets, events, teams)
├─ performances::Rows (performances + lookup refs)
├─ coverage::Classification (athletes, coaches, schools, meets, events, performances)
└─ projection::Report (athletes, coaches, schools)
```

## API Design

```rust
pub struct ExportDataset {
    pub athletes: Vec<CanonicalAthlete>,
    pub schools: BTreeMap<String, CanonicalSchool>,
    pub coaches: Vec<CanonicalCoach>,
    pub events: Vec<CanonicalEvent>,
    pub performances: Vec<CanonicalPerformance>,
    pub meets: Vec<CanonicalMeet>,
    pub teams: BTreeMap<String, CanonicalTeam>,
    pub generated_on: NaiveDate,
}

impl ExportDataset {
    pub fn load(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        // One scan per keyspace, apply scope/cohort filters, build in-memory indexes
    }
}
```

Consumers receive slices, not store access:

```rust
pub fn write_athletes(book: &mut Workbook, athletes: &[CanonicalAthlete], 
                      schools: &BTreeMap<String, CanonicalSchool>) -> ReportResult<()> { }

pub fn build_bests(performances: &[CanonicalPerformance], parents: &Parents,
                   options: &Options) -> Vec<SharedSelection> { }
```

## Benefits

- One decode per keyspace (athletes 1×, performances 1×, etc.)
- No repeated store.open() calls
- Consumers work on shared memory — faster, simpler
- Scope/cohort filtering done once during load, not per-consumer
- No store lock required during export — all data is in memory

## Migration Path

1. Add `ExportDataset` struct with `load` method
2. Update consumers to accept slices instead of store
3. Update `workbook::build` to load dataset once, pass to consumers
4. Remove store-scanning code from consumers
5. Keep store path as optional for backwards compatibility during transition
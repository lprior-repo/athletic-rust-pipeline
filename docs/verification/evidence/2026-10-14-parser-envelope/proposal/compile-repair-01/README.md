# First integrated compile correction

Root reported seven E0308 errors in the newly added outcome tests: `Store::journal_keys` returns `HashSet<String>`, while the exact-empty expectations used `Vec<String>`. Replace precisely those seven expectations with `std::collections::HashSet::<String>::new()`; no production semantics or assertion predicate changes.

Root owns the live correction and rerun. This directory preserves the formatted integrated before/after source and a contextual patch, with SHA-256 pins. The original detached candidate and f5b3988 context.patch remain unchanged as requested. The worker has not run a compiler or tests and claims no passing result here.

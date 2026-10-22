# Retained reported event contexts

These six unchanged response bodies were copied from the selected five-state
runtime's content-addressed HTTP archive on 2026-10-22. Each filename is the
SHA-256 of its bytes; the regression fixture checks that digest before use.
The original archive, Store, receipts and published generation remain unchanged.

| Body SHA-256 | Published source |
| --- | --- |
| e4fc22921eca6c4db1609a4a825755b9cea7c31c54983ac127372bd9255aa6b1 | MileSplit owned meet-performances API for meet 696747, USATF U20 Championships, June 19, 2025 |
| 150e47846964c02aa2aabc1d8f4d5bcd17dacfff3c62dd414932e5cc7fc534b9 | https://or.milesplit.com/meets/696747/results/1189674/raw |
| 5ee9c62e52458fdbc45e8ee5635009073b1552bfc7c0ccd275ac404c927a6d59 | MileSplit owned meet-performances API for meet 571515, Jim Boughton XC, September 21, 2023 |
| 121ea75901ecebeb5b249aaf6389514c59b59477871585b5c25c1e6b1dbfc101 | https://ia.milesplit.com/meets/571515/results/962542/raw |
| ee8dbefb41cfbb2316d94a9a2094ffe8c0b842af89b08fd1b3524a75cf3c56c4 | https://ia.milesplit.com/meets/571515/results/962544/raw |
| ed7b090f1c9ec811932f6d0c11563ebfb59a2ec49a09b0964f4dc642e5af16de | https://ia.milesplit.com/meets/571515/results/962541/raw |

Tests seed these bytes into an isolated offline fetch cache, create a temporary
Store and use the existing collector and retained-source qualification/replay
paths. The fixture cache timestamp and test school bindings are controlled test
inputs. They do not reproduce production receipts or establish accepted identity,
eligibility, school affiliation or complete source acquisition.

The Michigan API rows pair D with Discus and S with Shot Put. Its raw result text
lists the demonstrated athletes as Unattached while the API names schools; these
claims are retained without settling performance-time affiliation or implement
weight. The Illinois API names 5000 Meter Run while the raw meet explicitly names
Cross Country. Distance alone supplies neither a measured course nor a comparable
best. These examples qualify reported review annotations only.

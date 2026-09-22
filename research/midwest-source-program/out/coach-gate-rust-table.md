
## Classification of every row

Every row below was re-fetched from its own `source_url` cells. `verified` means a value
cell appeared in the page *and* the role label was corroborated within its window;
`role conveyed by page context` means the value appeared without that window; `render-required`
means the page is a JavaScript shell, so no value is reachable without executing script.
`robots-blocked` and `fetch failed` count pages the polite fetcher could not read at all —
those rows ship nowhere and the tallies say how many they are.

| fragment | rows | verified | role conveyed by page context | role contradicted | render-required | mismatch | empty | robots-blocked | fetch failed | shipped share |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/IA.csv` | 63 | 0 | 0 | 0 | 0 | 0 | 0 | 63 | 0 | 0.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/IL.csv` | 16 | 13 | 0 | 3 | 0 | 0 | 0 | 0 | 0 | 81.2 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/KS.csv` | 523 | 523 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/MI.csv` | 14 | 0 | 0 | 0 | 0 | 0 | 0 | 14 | 0 | 0.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/MN.csv` | 7 | 7 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/ND.csv` | 39 | 39 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/NE.csv` | 16 | 2 | 1 | 13 | 0 | 0 | 0 | 0 | 0 | 18.8 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/OH.csv` | 31 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 31 | 0.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/SD.csv` | 30 | 0 | 0 | 0 | 0 | 0 | 0 | 30 | 0 | 0.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments-ad/WI.csv` | 19 | 17 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/AZ.csv` | 188 | 171 | 0 | 11 | 5 | 1 | 0 | 0 | 0 | 91.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/CA.csv` | 193 | 152 | 0 | 41 | 0 | 0 | 0 | 0 | 0 | 78.8 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/CO.csv` | 200 | 200 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/DC.csv` | 50 | 36 | 4 | 0 | 10 | 0 | 0 | 0 | 0 | 80.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/FL.csv` | 4 | 1 | 3 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/GA.csv` | 193 | 0 | 42 | 151 | 0 | 0 | 0 | 0 | 0 | 21.8 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/IA.csv` | 93 | 62 | 20 | 0 | 0 | 0 | 0 | 0 | 11 | 88.2 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/IL.csv` | 197 | 170 | 0 | 27 | 0 | 0 | 0 | 0 | 0 | 86.3 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/IN.csv` | 130 | 106 | 13 | 11 | 0 | 0 | 0 | 0 | 0 | 91.5 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/KS.csv` | 37 | 11 | 22 | 4 | 0 | 0 | 0 | 0 | 0 | 89.2 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/MI.csv` | 106 | 82 | 11 | 12 | 1 | 0 | 0 | 0 | 0 | 87.7 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/MN.csv` | 1176 | 1173 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 99.7 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/MO.csv` | 61 | 54 | 6 | 0 | 1 | 0 | 0 | 0 | 0 | 98.4 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/ND.csv` | 199 | 141 | 0 | 58 | 0 | 0 | 0 | 0 | 0 | 70.9 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/NE.csv` | 209 | 75 | 22 | 92 | 20 | 0 | 0 | 0 | 0 | 46.4 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/NJ.csv` | 86 | 76 | 10 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/OH.csv` | 190 | 69 | 7 | 5 | 2 | 0 | 0 | 0 | 107 | 40.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/OR.csv` | 196 | 193 | 0 | 2 | 1 | 0 | 0 | 0 | 0 | 98.5 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/TN.csv` | 190 | 153 | 0 | 37 | 0 | 0 | 0 | 0 | 0 | 80.5 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/TX.csv` | 72 | 49 | 17 | 6 | 0 | 0 | 0 | 0 | 0 | 91.7 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/UT.csv` | 200 | 176 | 0 | 24 | 0 | 0 | 0 | 0 | 0 | 88.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/WI-wide.csv` | 5172 | 5172 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| `/home/lewis/Downloads/midwest-tfxc-source-research/out/coach-fragments/WI.csv` | 5172 | 5172 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 100.0 % |
| **total (33)** | **15072** | **14095** | **180** | **497** | **40** | **4** | **0** | **107** | **149** | **94.7 %** |

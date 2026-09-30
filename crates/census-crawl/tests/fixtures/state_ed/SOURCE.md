# `state_ed` captures

Byte-exact responses served by `data.nysed.gov` (New York State Education Department), copied with
`cp` from `census-prototype/raw/data.nysed.gov__<sha1-of-url>`. Nothing was reformatted or
re-serialized.

|Fixture|Corpus file|URL|Status/bytes|sha256|
|---|---|---|---|---|
|`index_letter_a.html`|`data.nysed.gov__58a5886b064eaca830473668`|not recorded by a sidecar; `https://data.nysed.gov/analysis/` the letter-A school index, inferred from the page's own title and links|200, 786,227|`adf44bb69c0c8751459c698162fe7526950a6e4feb1778a07d4f3c6c9aa37f27`|
|`profile_kingston.html`|`data.nysed.gov__5facfe61d423d8f0828d0bed`|`https://data.nysed.gov/profile.php?instid=800000038718`|200, 60,020|`6d720faec71b6dbf30eddb8045262e78daf62931d805a2a501f9eefa12442830`|

## What the captures pin

- `index_letter_a.html` — `<title>A - Schools | NYSED Data Site</title>`. The corpus holds the
  letter-A slice of the state's school index, not the whole index: the page carries **2,530**
  literal `instid=` occurrences, of which **2,528** have a numeric value, resolving to **220 distinct
  school ids**, all twelve digits, and **220 distinct `profile.php?instid=<id>` links** (220 link
  occurrences). The two nonnumeric markers are JavaScript's `instid=some_value` example and
  `instid=` string concatenation; they are not schools. The previous 221-school/link claim
  double-counted `800000054526`, which is already one of the 220 twelve-digit ids, and treated
  all marker occurrences as numeric. School names and institution ids come from the index;
  postal address components require the separately captured profile.
- `profile_kingston.html` — `<title>A A KINGSTON MIDDLE SCHOOL | NYSED Data Site</title>`, with the
  page's own `AT A GLANCE 2024-25` section. It is served from `profile.php?instid=800000038718`, so a
  profile is addressed by a 12-digit NYSED institution id that the index page supplies.
- No robots verdict for `data.nysed.gov` was captured in the corpus. The reader is a file reader and
  makes no request; a fetching lane would need its own admission decision, which the registry
  descriptor for `state_ed` records.

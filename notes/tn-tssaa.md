# Tennessee Secondary School Athletic Association (TSSAA)

**URL:** https://portal.tssaa.org/common/directory/
**Source type:** Custom HTML scraping (two-stage)

## Overview

TSSAA provides a searchable school directory listing ~456 member schools. Each school's detail page contains coach and staff information organized by sport.

## Data Structure

- **Directory listing page:** Contains an embedded JSON array of school IDs and names (used by the client-side typeahead search). No address, phone, or enrollment data is present on this page.
- **School detail pages:** Located at `https://portal.tssaa.org/common/directory/?id=<school_id>`. Each detail page shows:
  - School name, city, county, phone
  - Staff information for each sport the school fields a team in
  - Each sport has its own card: `<div id="sportN" name="sportN"></div>` anchor followed by `<div class="card">` with `<div class="card-header">` containing the sport name (e.g., "Boys' Cross Country") and `<div class="card-body">` containing the staff table.
  - Emails are obfuscated using JavaScript's `mail_hide()` function, which reverses the domain part.

## Parser Behavior

- The prototype tree's `parsers/tn_tssaa.py` (kind: custom:tn_tssaa) parses the directory listing, extracting school ID and name from the embedded JSON typeahead data. Each school record includes a `detail_url` field for the per-school detail page (required for coach data).
- The prototype tree's `parsers/tn_tssaa_school.py` (kind: custom:tn_tssaa_school; deleted from this tree by ADR-020 §6 and ported as `census-crawl::tssaa`) parses individual school detail pages. It uses BeautifulSoup to find all staff rows (`tr.staffPerson`), walks up to the containing `div.card`, and reads the card header to determine the sport. Staff rows from cards other than Track, CrossCountry, or AthleticDirector are dropped (this census only requires Track/CrossCountry coaches). Email addresses are decoded from the `mail_hide()` obfuscation by reversing the domain argument.

## Sports Filtered

- **Included:** Track (1,154 rows), CrossCountry (1,118 rows), AthleticDirector (497 rows)
- **Dropped:** Football, Baseball, Basketball, Soccer, Softball, Tennis, Golf, Swimming, Wrestling, Volleyball, Cheer, Band, Marching Band, and other sports listed on TSSAA detail pages.
- Schools with at least one Track or CrossCountry row: 384

## Results

- **Schools:** 456
- **Coach rows:** 2,769
- **Schools with Track/CrossCountry coaches:** 384

## Limitations

- No address data available on TSSAA website.
- No enrollment data available on TSSAA website.
- Email decoding is required (reversing domain) — decoded emails are the actual contact addresses, not literal substrings from the page.
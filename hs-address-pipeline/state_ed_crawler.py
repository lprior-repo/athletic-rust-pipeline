"""State Education Agency Data Fetcher.

Aggregates school directory data from state education agencies.
Each state may have a different data format (CSV, JSON, API, HTML).
"""

import requests
import csv
import io
import json
import logging
from pathlib import Path
from urllib.parse import urljoin, urlparse
from html.parser import HTMLParser

logger = logging.getLogger(__name__)

# State-specific download URLs (examples)
STATE_DATA_URLS = {
    "CA": "https://www.cde.ca.gov/data/schools/school_directory.csv",
    "TX": "https://tea.texas.gov/data/schools/schools.csv",
    "NY": "https://data.nysed.gov/api/views/pk6t-98y5/rows.csv",
    "FL": "https://www.fldoe.org/data/schools/school_directory.csv",
    "PA": "https://www.education.pa.gov/data/schools.csv",
}

class SchoolDirectoryParser(HTMLParser):
    """Parser for HTML-based school directories."""
    
    def __init__(self):
        super().__init__()
        self.schools = []
        self.current_school = {}
        self.in_school = False
        self.current_field = None
        
    def handle_starttag(self, tag, attrs):
        if tag == 'tr' and self.in_school:
            pass  # Row started
        elif tag == 'td':
            self.current_field = tag
            self.current_school[tag] = ""
    
    def handle_data(self, data):
        if self.current_field:
            self.current_school[self.current_field] = data.strip()
    
    def handle_endtag(self, tag):
        if tag == 'td':
            self.current_field = None
        elif tag == 'tr' and self.current_school:
            self.schools.append(self.current_school)
            self.current_school = {}

def download_state_data(state_code, url=None):
    """Download school directory data for a specific state.
    
    Args:
        state_code: Two-letter state code
        url: Optional URL to override the default
    
    Returns:
        Content (file object or string)
    """
    if url is None:
        url = STATE_DATA_URLS.get(state_code)
        if not url:
            logger.warning(f"No URL configured for state {state_code}")
            return None
    
    logger.info(f"Downloading school data for {state_code} from {url}")
    
    try:
        resp = requests.get(url, timeout=60)
        resp.raise_for_status()
        return resp.content
    except requests.RequestException as e:
        logger.error(f"Failed to download data for {state_code}: {e}")
        return None

def parse_csv_data(content, state_code):
    """Parse CSV-format state data."""
    schools = []
    
    try:
        if isinstance(content, bytes):
            text = content.decode('utf-8')
        else:
            text = content
        
        # Split by newlines, skip header
        lines = text.strip().split('\n')
        for line in lines[1:]:  # Skip header
            parts = line.split(',')
            if len(parts) >= 6:
                school = {
                    'state': state_code,
                    'unit_id': parts[0],
                    'name': parts[1],
                    'address': parts[2],
                    'city': parts[3],
                    'zip_code': parts[4],
                    'type': parts[5] if len(parts) > 5 else 'Public',
                    'enrollment': parts[6] if len(parts) > 6 else '',
                }
                schools.append(school)
    except Exception as e:
        logger.error(f"Failed to parse CSV data for {state_code}: {e}")
    
    return schools

def parse_json_data(content, state_code):
    """Parse JSON-format state data."""
    schools = []
    
    try:
        if isinstance(content, bytes):
            data = json.loads(content.decode('utf-8'))
        else:
            data = json.loads(content)
        
        # Handle different JSON structures
        if isinstance(data, list):
            items = data
        elif isinstance(data, dict):
            # Try common field names
            for key in ['schools', 'data', 'results', 'items']:
                if key in data:
                    items = data[key]
                    break
            else:
                logger.warning(f"Unknown JSON structure for {state_code}")
                return []
        
        for item in items:
            school = {
                'state': state_code,
                'unit_id': item.get('unit_id', item.get('id', '')),
                'name': item.get('name', ''),
                'address': item.get('address', ''),
                'city': item.get('city', ''),
                'zip_code': item.get('zip_code', item.get('zipcode', '')),
                'type': item.get('type', 'Public'),
                'enrollment': item.get('enrollment', ''),
            }
            schools.append(school)
    except Exception as e:
        logger.error(f"Failed to parse JSON data for {state_code}: {e}")
    
    return schools

def fetch_all_state_data():
    """Fetch school data for all configured states."""
    all_schools = []
    
    for state_code, url in STATE_DATA_URLS.items():
        try:
            content = download_state_data(state_code, url)
            if content:
                # Try CSV first, then JSON
                if isinstance(content, str) and content.strip().startswith('{'):
                    schools = parse_json_data(content, state_code)
                else:
                    schools = parse_csv_data(content, state_code)
                
                logger.info(f"Parsed {len(schools)} schools for {state_code}")
                all_schools.extend(schools)
        except Exception as e:
            logger.error(f"Error processing {state_code}: {e}")
    
    logger.info(f"Total state schools fetched: {len(all_schools)}")
    return all_schools

def main():
    """Run the State ED data fetcher."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    schools = fetch_all_state_data()
    
    # Write output to CSV
    output_path = Path("output/state_ed_schools.csv")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', newline='') as f:
        if schools:
            writer = csv.DictWriter(f, fieldnames=schools[0].keys())
            writer.writeheader()
            writer.writerows(schools)
    
    logger.info(f"Written {len(schools)} state schools to {output_path}")
    return schools

if __name__ == "__main__":
    main()

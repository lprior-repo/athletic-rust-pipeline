"""NCES Common Core of Data (CCD) crawler.

Fetches public school data from NCES's primary database.
"""

import requests
import zipfile
import gzip
import csv
import io
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

NCES_CCD_SCHOOL_URL = "https://nces.ed.gov/ccd/schools/"
NCES_CCD_STATE_URL = "https://nces.ed.gov/ccd/schools/ccdstate.csv"

def get_state_codes():
    """Fetch the FIPS state codes table from NCES."""
    try:
        resp = requests.get(NCES_CCD_STATE_URL, timeout=30)
        resp.raise_for_status()
        return resp.content
    except requests.RequestException as e:
        logger.error(f"Failed to fetch state codes: {e}")
        return None

def get_schools_by_state(state_code, year="latest"):
    """Download the schools file for a specific state.
    
    Args:
        state_code: Two-digit FIPS state code (e.g., '36' for New York)
        year: Academic year or 'latest'
    
    Returns:
        File object with the CSV content
    """
    if year == "latest":
        url = f"{NCES_CCD_SCHOOL_URL}ccd_schools_{state_code}.csv.gz"
    else:
        url = f"{NCES_CCD_SCHOOL_URL}ccd{year}_schools_{state_code}.csv.gz"
    
    logger.info(f"Downloading schools data for state {state_code} from {url}")
    
    try:
        resp = requests.get(url, timeout=120)
        resp.raise_for_status()
        
        # Decompress gzip content
        decompressed = gzip.decompress(resp.content)
        return io.BytesIO(decompressed)
    except requests.RequestException as e:
        logger.error(f"Failed to download schools data for state {state_code}: {e}")
        return None

def parse_schools_csv(file_obj):
    """Parse the CCD schools CSV file.
    
    The file contains school-level data with the following columns:
    - UnitID: NCES school ID
    - Sector: Public/Private indicator
    - SchoolName: School name
    - Address1: Street address
    - City: City name
    - State: Two-letter state code
    - ZipCode: ZIP code
    - DistrictID: District NCES ID
    - GradeLow: Lowest grade level
    - GradeHigh: Highest grade level
    - Enrollment: Total enrollment
    - FreeLunch: Free/reduced lunch count
    - TotalTeachers: Number of teachers
    
    Returns:
        List of parsed school dictionaries
    """
    schools = []
    
    try:
        # Decode from bytes to string
        if isinstance(file_obj, io.BytesIO):
            text = file_obj.read().decode('utf-8')
        else:
            text = file_obj.read()
        
        # Parse CSV (skip empty lines)
        lines = text.split('\n')
        for line in lines:
            if not line.strip() or line.startswith('"'):
                # Skip header or empty lines
                continue
            
            parts = line.split(',')
            if len(parts) < 8:
                continue
            
            school = {
                'unit_id': parts[0],
                'sector': parts[1],
                'name': parts[2],
                'address': parts[3],
                'city': parts[4],
                'state': parts[5],
                'zip_code': parts[6],
                'district_id': parts[7],
                'grade_low': parts[8] if len(parts) > 8 else '',
                'grade_high': parts[9] if len(parts) > 9 else '',
                'enrollment': parts[10] if len(parts) > 10 else '',
            }
            schools.append(school)
    
    except Exception as e:
        logger.error(f"Failed to parse schools CSV: {e}")
    
    return schools

def fetch_all_schools():
    """Fetch school data for all states.
    
    Returns:
        List of all school dictionaries
    """
    all_schools = []
    state_codes_content = get_state_codes()
    
    if state_codes_content is None:
        logger.warning("Could not fetch state codes, using hardcoded list")
        state_codes = [str(i).zfill(2) for i in range(1, 57)]  # 1-50, 60-78, 79
    else:
        # Parse state codes from content
        state_codes = []
        for line in state_codes_content.decode('utf-8').split('\n'):
            parts = line.strip().split('\t')
            if parts:
                state_codes.append(parts[0])
    
    logger.info(f"Fetching school data for {len(state_codes)} states")
    
    for state_code in state_codes:
        try:
            file_obj = get_schools_by_state(state_code)
            if file_obj:
                schools = parse_schools_csv(file_obj)
                logger.info(f"Parsed {len(schools)} schools for state {state_code}")
                all_schools.extend(schools)
        except Exception as e:
            logger.error(f"Error processing state {state_code}: {e}")
    
    logger.info(f"Total schools fetched: {len(all_schools)}")
    return all_schools

def main():
    """Run the NCES CCD data fetcher."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    schools = fetch_all_schools()
    
    # Write output to CSV
    output_path = Path("output/ccd_schools.csv")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', newline='') as f:
        if schools:
            writer = csv.DictWriter(f, fieldnames=schools[0].keys())
            writer.writeheader()
            writer.writerows(schools)
    
    logger.info(f"Written {len(schools)} schools to {output_path}")
    return schools

if __name__ == "__main__":
    main()

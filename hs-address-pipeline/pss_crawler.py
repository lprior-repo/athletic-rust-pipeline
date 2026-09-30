"""NCES Private School Universe Survey (PSS) crawler.

Fetches private school data from NCES's biennial survey of private K-12 schools.
"""

import requests
import gzip
import csv
import io
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

NCES_PSS_BASE_URL = "https://nces.ed.gov/pss/"
NCES_PSS_SCHOOLS_URL = "https://nces.ed.gov/pss/pss_schools.csv.gz"
NCES_PSS_INSTITUTIONS_URL = "https://nces.ed.gov/pss/pss_institutions.csv.gz"

def get_private_schools(year="latest"):
    """Download the private schools file from NCES PSS.
    
    The PSS file contains school-level data with the following columns:
    - InstitutionID: NCES institution ID
    - InstitutionName: School name
    - Street: Street address
    - City: City name
    - State: Two-letter state code
    - ZipCode: ZIP code
    - GradeRange: Grade range (e.g., K-12)
    - SchoolType: Type (e.g., Religious, Secular)
    - Enrollment: Total enrollment
    - ReligiousAffiliation: Religious denomination if applicable
    
    Args:
        year: Academic year or 'latest'
    
    Returns:
        File object with the CSV content
    """
    if year == "latest":
        url = NCES_PSS_SCHOOLS_URL
    else:
        url = f"https://nces.ed.gov/pss/pss{year}_schools.csv.gz"
    
    logger.info(f"Downloading private schools data from {url}")
    
    try:
        resp = requests.get(url, timeout=120)
        resp.raise_for_status()
        
        # Decompress gzip content
        decompressed = gzip.decompress(resp.content)
        return io.BytesIO(decompressed)
    except requests.RequestException as e:
        logger.error(f"Failed to download private schools data: {e}")
        return None

def parse_private_schools_csv(file_obj):
    """Parse the PSS private schools CSV file.
    
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
        
        # Parse CSV (skip header)
        lines = text.split('\n')
        for line in lines:
            if not line.strip():
                continue
            
            parts = line.split(',')
            if len(parts) < 8:
                continue
            
            school = {
                'institution_id': parts[0],
                'name': parts[1],
                'street': parts[2],
                'city': parts[3],
                'state': parts[4],
                'zip_code': parts[5],
                'grade_range': parts[6] if len(parts) > 6 else '',
                'school_type': parts[7] if len(parts) > 7 else '',
                'enrollment': parts[8] if len(parts) > 8 else '',
                'religious_affiliation': parts[9] if len(parts) > 9 else '',
            }
            schools.append(school)
    
    except Exception as e:
        logger.error(f"Failed to parse private schools CSV: {e}")
    
    return schools

def fetch_all_private_schools():
    """Fetch all private school data from NCES PSS.
    
    Returns:
        List of all private school dictionaries
    """
    try:
        file_obj = get_private_schools()
        if file_obj:
            schools = parse_private_schools_csv(file_obj)
            logger.info(f"Parsed {len(schools)} private schools")
            return schools
    except Exception as e:
        logger.error(f"Error processing private schools data: {e}")
    
    return []

def main():
    """Run the NCES PSS data fetcher."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    schools = fetch_all_private_schools()
    
    # Write output to CSV
    output_path = Path("output/pss_private_schools.csv")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', newline='') as f:
        if schools:
            writer = csv.DictWriter(f, fieldnames=schools[0].keys())
            writer.writeheader()
            writer.writerows(schools)
    
    logger.info(f"Written {len(schools)} private schools to {output_path}")
    return schools

if __name__ == "__main__":
    main()

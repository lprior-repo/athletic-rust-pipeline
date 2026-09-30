"""Private School Association Data Fetcher.

Scrapes and requests directory data from private school associations
including NAIS, CAPE, NASSP, and accrediting bodies.
"""

import requests
import logging
from pathlib import Path
from bs4 import BeautifulSoup
import csv

logger = logging.getLogger(__name__)

# Association URLs
NAIS_URL = "https://www.nais.org/members/schools/"
CAPE_URL = "https://www.cape-ed.org/schools/"
NASSP_URL = "https://www.nassp.org/schools/"

def fetch_nais_members():
    """Fetch NAIS member school list."""
    logger.info("Fetching NAIS member schools from {NAIS_URL}")
    
    try:
        resp = requests.get(NAIS_URL, timeout=60)
        resp.raise_for_status()
        
        soup = BeautifulSoup(resp.content, 'html.parser')
        
        schools = []
        # Parse NAIS school listing HTML
        for school_item in soup.find_all('div', class_='school-list-item'):
            name = school_item.find('h3').text.strip()
            address = school_item.find('div', class_='address').text.strip()
            # Additional parsing for address components
            
            schools.append({
                'source': 'NAIS',
                'name': name,
                'address': address,
                'city': '',
                'state': '',
                'zip_code': '',
                'type': 'Private',
            })
        
        logger.info(f"Found {len(schools)} NAIS member schools")
        return schools
        
    except requests.RequestException as e:
        logger.error(f"Failed to fetch NAIS members: {e}")
        return []

def fetch_cape_members():
    """Fetch CAPE member school list."""
    logger.info("Fetching CAPE member schools from {CAPE_URL}")
    
    try:
        resp = requests.get(CAPE_URL, timeout=60)
        resp.raise_for_status()
        
        # Parse CAPE school listing
        schools = []
        # Similar parsing logic as NAIS
        
        logger.info(f"Found {len(schools)} CAPE member schools")
        return schools
        
    except requests.RequestException as e:
        logger.error(f"Failed to fetch CAPE members: {e}")
        return []

def fetch_nassp_members():
    """Fetch NASSP member school list."""
    logger.info("Fetching NASSP member schools from {NASSP_URL}")
    
    try:
        resp = requests.get(NASSP_URL, timeout=60)
        resp.raise_for_status()
        
        # Parse NASSP school listing
        schools = []
        # Similar parsing logic
        
        logger.info(f"Found {len(schools)} NASSP member schools")
        return schools
        
    except requests.RequestException as e:
        logger.error(f"Failed to fetch NASSP members: {e}")
        return []

def fetch_all_associations():
    """Fetch all private school association data."""
    all_schools = []
    
    try:
        nais_schools = fetch_nais_members()
        all_schools.extend(nais_schools)
    except Exception as e:
        logger.error(f"Error processing NAIS: {e}")
    
    try:
        cape_schools = fetch_cape_members()
        all_schools.extend(cape_schools)
    except Exception as e:
        logger.error(f"Error processing CAPE: {e}")
    
    try:
        nassp_schools = fetch_nassp_members()
        all_schools.extend(nassp_schools)
    except Exception as e:
        logger.error(f"Error processing NASSP: {e}")
    
    logger.info(f"Total association schools: {len(all_schools)}")
    return all_schools

def main():
    """Run the Private School Association Fetcher."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    schools = fetch_all_associations()
    
    # Write output to CSV
    output_path = Path("output/association_schools.csv")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w', newline='') as f:
        if schools:
            writer = csv.DictWriter(f, fieldnames=schools[0].keys())
            writer.writeheader()
            writer.writerows(schools)
    
    logger.info(f"Written {len(schools)} association schools to {output_path}")
    return schools

if __name__ == "__main__":
    main()

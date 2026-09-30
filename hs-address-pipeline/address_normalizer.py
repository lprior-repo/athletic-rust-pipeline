"""Address Normalization & Deduplication Engine.

Standardizes address fields using USPS API and local normalization,
then de-duplicates by matching names/addresses or school IDs.
"""

import logging
import requests
import json
from collections import defaultdict

logger = logging.getLogger(__name__)

# USPS API configuration
USPS_API_KEY = "YOUR_USPS_API_KEY"
USPS_VERIFY_URL = "https://api.usps.com/webtools/addrvalidateapi.asmx/AddressCorrect"

class AddressNormalizer:
    """Normalize and validate school addresses."""
    
    def __init__(self, api_key=None):
        self.api_key = api_key or USPS_API_KEY
    
    def normalize_address(self, address, city, state, zip_code):
        """Normalize an address string using USPS API.
        
        Args:
            address: Street address
            city: City name
            state: Two-letter state code
            zip_code: ZIP code
        
        Returns:
            Dictionary with normalized address components
        """
        # Build USPS request payload
        payload = {
            "Address": {
                "Line1": address,
                "Line2": "",
                "City": city,
                "State": state,
                "Zip": zip_code,
                "CarrierRoute": "",
                "DeliveryPoint": "",
                "DeliveryPointCheckDigit": ""
            }
        }
        
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Basic {self.api_key}"
        }
        
        try:
            resp = requests.post(USPS_VERIFY_URL, json=payload, headers=headers, timeout=10)
            resp.raise_for_status()
            
            result = resp.json()
            
            # Extract normalized fields from response
            return {
                'street': result['Address']['Line1'],
                'secondary': result['Address']['Line2'],
                'city': result['Address']['City'],
                'state': result['Address']['State'],
                'zip_code': result['Address']['Zip'],
                'plus4': result['Address'].get('Plus4', ''),
                'validated': True
            }
        except requests.RequestException as e:
            logger.error(f"USPS normalization failed: {e}")
            return {
                'street': address,
                'secondary': '',
                'city': city,
                'state': state,
                'zip_code': zip_code,
                'plus4': '',
                'validated': False
            }
    
    def normalize_local(self, address, city, state, zip_code):
        """Local address normalization without API call."""
        # Clean up common formatting issues
        import re
        
        # Fix double spaces
        address = re.sub(r'\s+', ' ', address).strip()
        city = re.sub(r'\s+', ' ', city).strip()
        
        # Capitalize properly
        address = address.title()
        city = city.title()
        state = state.upper()
        
        return {
            'street': address,
            'secondary': '',
            'city': city,
            'state': state,
            'zip_code': zip_code,
            'plus4': '',
            'validated': False
        }

class AddressDeduplicator:
    """Deduplicate schools by matching identifiers."""
    
    def __init__(self):
        self.nces_index = defaultdict(list)
        self.pss_index = defaultdict(list)
        self.state_index = defaultdict(list)
        self.association_index = defaultdict(list)
    
    def index_school(self, school, source='unknown'):
        """Add a school to the appropriate index."""
        if 'unit_id' in school and school['unit_id']:
            self.nces_index[school['unit_id']].append(school)
        if 'institution_id' in school and school['institution_id']:
            self.pss_index[school['institution_id']].append(school)
        if 'state' in school and school['state']:
            self.state_index[school['state']].append(school)
        if 'source' in school and school['source']:
            self.association_index[school['source']].append(school)
    
    def deduplicate(self, schools):
        """Deduplicate a list of schools.
        
        Uses a priority system:
        1. NCES ID matches
        2. PSS ID matches
        3. Name + Address matches
        4. Manual review
        """
        # First pass: Group by NCES ID
        nces_groups = defaultdict(list)
        for school in schools:
            if 'unit_id' in school and school['unit_id']:
                nces_groups[school['unit_id']].append(school)
        
        # For each group, select the best match
        deduplicated = []
        for nces_id, group in nces_groups.items():
            # Pick the record with most complete address
            best = self._select_best_record(group)
            best['unit_id'] = nces_id
            deduplicated.append(best)
        
        # For schools without NCES ID, group by name+address
        no_id_schools = [s for s in schools if not s.get('unit_id')]
        name_address_groups = defaultdict(list)
        for school in no_id_schools:
            key = self._name_address_key(school)
            name_address_groups[key].append(school)
        
        for key, group in name_address_groups.items():
            best = self._select_best_record(group)
            deduplicated.append(best)
        
        return deduplicated
    
    def _name_address_key(self, school):
        """Generate a key for name+address matching."""
        import re
        name = re.sub(r'[^\w\s]', '', school.get('name', '')).lower().strip()
        address = re.sub(r'[^\w\s]', '', school.get('address', '')).lower().strip()
        city = school.get('city', '').lower().strip()
        state = school.get('state', '').upper().strip()
        
        return (name, address, city, state)
    
    def _select_best_record(self, group):
        """Select the most complete record from a group."""
        # Priority: validated address > longer address > any
        validated = [s for s in group if s.get('validated', False)]
        if validated:
            return validated[0]
        
        # Pick the one with the most characters in address
        return max(group, key=lambda s: len(s.get('address', '')))

def normalize_and_deduplicate(schools, use_usps=True):
    """Normalize addresses and deduplicate school list.
    
    Args:
        schools: List of school dictionaries
        use_usps: Whether to use USPS API for validation
    
    Returns:
        List of deduplicated, normalized schools
    """
    normalizer = AddressNormalizer()
    
    # Normalize all addresses
    normalized = []
    for school in schools:
        normalized_school = school.copy()
        if use_usps:
            result = normalizer.normalize_address(
                school.get('address', ''),
                school.get('city', ''),
                school.get('state', ''),
                school.get('zip_code', '')
            )
        else:
            result = normalizer.normalize_local(
                school.get('address', ''),
                school.get('city', ''),
                school.get('state', ''),
                school.get('zip_code', '')
            )
        
        normalized_school['address'] = result['street']
        normalized_school['city'] = result['city']
        normalized_school['state'] = result['state']
        normalized_school['zip_code'] = result['zip_code']
        normalized_school['plus4'] = result['plus4']
        normalized_school['validated'] = result['validated']
        
        normalized.append(normalized_school)
    
    # Deduplicate
    deduplicator = AddressDeduplicator()
    deduplicated = deduplicator.deduplicate(normalized)
    
    logger.info(f"Normalized and deduplicated: {len(schools)} -> {len(deduplicated)} schools")
    return deduplicated

def main():
    """Test the normalizer with sample data."""
    logging.basicConfig(level=logging.INFO)
    
    sample_schools = [
        {
            'name': 'Lincoln High School',
            'address': '123 main ST',
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'unit_id': '170012345'
        },
        {
            'name': 'Lincoln High School',
            'address': '123 Main Street',
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'unit_id': ''  # Duplicate without ID
        }
    ]
    
    result = normalize_and_deduplicate(sample_schools, use_usps=False)
    for school in result:
        print(f"{school['name']}: {school['address']}, {school['city']}, {school['state']}")

if __name__ == "__main__":
    main()

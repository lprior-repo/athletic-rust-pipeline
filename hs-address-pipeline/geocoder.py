"""Geocoding & Address Validation.

Verifies addresses and obtains coordinates using Google Maps Geocoding API
and/or USPS validation services.
"""

import logging
import requests
import time
import csv
import json
from pathlib import Path
from typing import List, Dict, Optional

logger = logging.getLogger(__name__)

# API configuration
GOOGLE_API_KEY = "YOUR_GOOGLE_API_KEY"
GOOGLE_GEOCODE_URL = "https://maps.googleapis.com/maps/api/geocode/json"
USPS_API_KEY = "YOUR_USPS_API_KEY"
USPS_VERIFY_URL = "https://api.usps.com/webtools/addrvalidateapi.asmx/AddressCorrect"

class Geocoder:
    """Geocode school addresses and obtain coordinates."""
    
    def __init__(self, google_api_key: Optional[str] = None, 
                 usps_api_key: Optional[str] = None):
        self.google_api_key = google_api_key or GOOGLE_API_KEY
        self.usps_api_key = usps_api_key or USPS_API_KEY
    
    def geocode_address(self, address: str, city: str, state: str, 
                       zip_code: str) -> Dict:
        """Geocode a single address using Google Maps API.
        
        Args:
            address: Street address
            city: City name
            state: Two-letter state code
            zip_code: ZIP code
        
        Returns:
            Dictionary with geocode result including lat/lng
        """
        # Build the query string
        query = f"{address}, {city}, {state} {zip_code}"
        
        url = f"{GOOGLE_GEOCODE_URL}?address={query}&key={self.google_api_key}"
        
        try:
            resp = requests.get(url, timeout=10)
            resp.raise_for_status()
            
            result = resp.json()
            
            if result['status'] == 'OK' and result['results']:
                location = result['results'][0]['geometry']['location']
                return {
                    'latitude': location['lat'],
                    'longitude': location['lng'],
                    'formatted_address': result['results'][0]['formatted_address'],
                    'accuracy': result['results'][0].get('accuracy', 'ROOFTOP'),
                    'geocode_status': result['status']
                }
            else:
                return {
                    'latitude': None,
                    'longitude': None,
                    'formatted_address': '',
                    'accuracy': '',
                    'geocode_status': result['status']
                }
        except requests.RequestException as e:
            logger.error(f"Geocoding failed: {e}")
            return {
                'latitude': None,
                'longitude': None,
                'formatted_address': '',
                'accuracy': '',
                'geocode_status': 'ERROR'
            }
    
    def geocode_batch(self, schools: List[Dict], delay: float = 0.1) -> List[Dict]:
        """Geocode a batch of schools.
        
        Args:
            schools: List of school dictionaries
            delay: Delay between API calls (seconds)
        
        Returns:
            List of schools with geocode results
        """
        geocoded = []
        total = len(schools)
        
        for i, school in enumerate(schools):
            try:
                result = self.geocode_address(
                    school.get('address', ''),
                    school.get('city', ''),
                    school.get('state', ''),
                    school.get('zip_code', '')
                )
                
                # Add geocode results to school record
                geocoded_school = school.copy()
                geocoded_school['latitude'] = result['latitude']
                geocoded_school['longitude'] = result['longitude']
                geocoded_school['formatted_address'] = result['formatted_address']
                geocoded_school['geocode_status'] = result['geocode_status']
                geocoded_school['geocode_accuracy'] = result['accuracy']
                
                geocoded.append(geocoded_school)
                
                # Log progress every 100 schools
                if (i + 1) % 100 == 0:
                    logger.info(f"Geocoded {i + 1}/{total} schools")
                
                # Respect API rate limits
                if i < total - 1:
                    time.sleep(delay)
                    
            except Exception as e:
                logger.error(f"Error geocoding school {school['name']}: {e}")
                geocoded.append(school)
        
        logger.info(f"Completed geocoding {len(geocoded)} schools")
        return geocoded


class USPSAddressValidator:
    """Validate addresses using USPS API."""
    
    def __init__(self, api_key: Optional[str] = None):
        self.api_key = api_key or USPS_API_KEY
    
    def validate_address(self, address: str, city: str, state: str, 
                        zip_code: str) -> Dict:
        """Validate an address using USPS API."""
        payload = {
            "Address": {
                "Line1": address,
                "City": city,
                "State": state,
                "Zip": zip_code
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
            
            return {
                'validated': True,
                'primary_number': result['Address'].get('PrimaryNumber', ''),
                'street_name': result['Address'].get('StreetName', ''),
                'street_suffix': result['Address'].get('StreetSuffix', ''),
                'city': result['Address'].get('City', city),
                'state': result['Address'].get('State', state),
                'zip': result['Address'].get('Zip', zip_code),
            }
        except requests.RequestException as e:
            logger.error(f"USPS validation failed: {e}")
            return {'validated': False, 'error': str(e)}


def geocode_and_validate(schools: List[Dict], 
                         use_google: bool = True,
                         use_usps: bool = True) -> List[Dict]:
    """Geocode and validate all schools.
    
    Args:
        schools: List of school dictionaries
        use_google: Use Google Maps for geocoding
        use_usps: Use USPS for address validation
    
    Returns:
        List of validated schools
    """
    if use_google:
        geocoder = Geocoder()
        geocoded = geocoder.geocode_batch(schools)
    else:
        geocoded = schools
    
    if use_usps:
        validator = USPSAddressValidator()
        validated = []
        for school in geocoded:
            result = validator.validate_address(
                school.get('address', ''),
                school.get('city', ''),
                school.get('state', ''),
                school.get('zip_code', '')
            )
            validated_school = school.copy()
            validated_school.update(result)
            validated.append(validated_school)
        return validated
    
    return geocoded


def main():
    """Test geocoding with sample data."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    # Sample school data
    test_schools = [
        {
            'name': 'Lincoln High School',
            'address': '123 Main St',
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'unit_id': '170012345'
        }
    ]
    
    # Only run if API key is set
    if GOOGLE_API_KEY != "YOUR_GOOGLE_API_KEY":
        result = geocode_and_validate(test_schools, use_usps=False)
        
        for school in result:
            print(f"{school['name']}: {school.get('latitude')}, {school.get('longitude')}")
            print(f"  Formatted: {school.get('formatted_address', 'N/A')}")
    else:
        print("Set GOOGLE_API_KEY environment variable to run geocoding test")

if __name__ == "__main__":
    main()

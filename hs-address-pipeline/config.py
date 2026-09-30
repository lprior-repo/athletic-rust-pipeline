"""Configuration for the High School Address Pipeline."""

import os

# NCES URLs
NCES_CCD_BASE_URL = "https://nces.ed.gov/ccd/schools/"
NCES_CCD_SCHOOL_CSV_URL = "https://nces.ed.gov/ccd/schools/ccd{year}_school.csv.gz"
NCES_PSS_BASE_URL = "https://nces.ed.gov/pss/"
NCES_PSS_CSV_URL = "https://nces.ed.gov/pss/pss{year}_schools.csv.gz"

# USPS API
USPS_API_KEY = os.environ.get("USPS_API_KEY", "")
USPS_VERIFY_URL = "https://api.usps.com/webtools/addrvalidateapi.asmx/AddressCorrect"

# Google Maps API
GOOGLE_MAPS_API_KEY = os.environ.get("GOOGLE_MAPS_API_KEY", "")
GOOGLE_GEOCODE_URL = "https://maps.googleapis.com/maps/api/geocode/json"

# State Education Agency URLs (abbreviated list)
STATE_ED_URLS = {
    "CA": "https://www.cde.ca.gov/data/",
    "TX": "https://tea.texas.gov/data/",
    "NY": "https://data.nysed.gov/",
    "FL": "https://www.fldoe.org/data/",
    "PA": "https://www.education.pa.gov/",
    # ... all 50 states
}

# Output settings
OUTPUT_DIR = os.environ.get("PIPELINE_OUTPUT_DIR", "hs-address-pipeline/output")
BATCH_SIZE = 100
REQUEST_TIMEOUT = 30
RETRY_ATTEMPTS = 3

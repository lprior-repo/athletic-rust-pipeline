"""Update Strategy & Change Detection.

Manages periodic re-fetch schedules and detects changes in school data
between updates.
"""

import logging
import json
import hashlib
from datetime import datetime, timedelta
from pathlib import Path
from typing import List, Dict, Optional

logger = logging.getLogger(__name__)

# Update frequency configuration
UPDATE_SCHEDULES = {
    'NCES_CCD': {'frequency': 'yearly', 'month': 9},  # September
    'NCES_PSS': {'frequency': 'biennial', 'year_mod': 0},  # Even years
    'STATE_ED': {'frequency': 'semester', 'months': [1, 7]},  # Jan & Jul
    'PRIVATE_ASSOC': {'frequency': 'quarterly', 'months': [1, 4, 7, 10]},
}

class ChangeDetector:
    """Detect changes between school data updates."""
    
    def __init__(self, baseline_path: str = None):
        self.baseline_path = baseline_path
        self.baseline = None
    
    def load_baseline(self, path: Optional[str] = None):
        """Load the baseline school data for comparison."""
        load_path = path or self.baseline_path
        if not load_path or not Path(load_path).exists():
            logger.warning(f"No baseline data at {load_path}")
            return False
        
        try:
            with open(load_path, 'r') as f:
                self.baseline = json.load(f)
            logger.info(f"Loaded baseline with {len(self.baseline)} schools")
            return True
        except json.JSONDecodeError as e:
            logger.error(f"Failed to load baseline: {e}")
            return False
    
    def save_current_as_baseline(self, schools: List[Dict], 
                                path: Optional[str] = None):
        """Save current data as the new baseline."""
        save_path = path or self.baseline_path
        if not save_path:
            save_path = "output/baseline.json"
        
        try:
            with open(save_path, 'w') as f:
                json.dump(schools, f, indent=2)
            logger.info(f"Saved baseline with {len(schools)} schools to {save_path}")
            return True
        except Exception as e:
            logger.error(f"Failed to save baseline: {e}")
            return False
    
    def detect_changes(self, new_schools: List[Dict]) -> Dict:
        """Detect changes between baseline and new data.
        
        Returns:
            Dictionary with added, removed, and modified schools
        """
        if not self.baseline:
            logger.warning("No baseline loaded; returning all schools as new")
            return {
                'added': new_schools,
                'removed': [],
                'modified': []
            }
        
        # Build lookup by ID
        baseline_by_id = {}
        for school in self.baseline:
            key = self._school_key(school)
            baseline_by_id[key] = school
        
        new_by_id = {}
        for school in new_schools:
            key = self._school_key(school)
            new_by_id[key] = school
        
        added = []
        removed = []
        modified = []
        
        # Find added schools
        for key, school in new_by_id.items():
            if key not in baseline_by_id:
                added.append(school)
        
        # Find removed schools
        for key, school in baseline_by_id.items():
            if key not in new_by_id:
                removed.append(school)
        
        # Find modified schools
        for key in new_by_id:
            if key in baseline_by_id:
                old = baseline_by_id[key]
                new = new_by_id[key]
                if self._school_changed(old, new):
                    modified.append({
                        'old': old,
                        'new': new,
                        'changes': self._get_changes(old, new)
                    })
        
        logger.info(f"Changes detected: {len(added)} added, {len(removed)} removed, {len(modified)} modified")
        
        return {
            'added': added,
            'removed': removed,
            'modified': modified
        }
    
    def _school_key(self, school: Dict) -> str:
        """Generate a unique key for a school."""
        # Prefer ID, fall back to name+address
        if school.get('unit_id'):
            return f"nces_{school['unit_id']}"
        elif school.get('institution_id'):
            return f"pss_{school['institution_id']}"
        else:
            name = school.get('name', '').lower().strip()
            address = school.get('address', '').lower().strip()
            city = school.get('city', '').lower().strip()
            state = school.get('state', '').upper().strip()
            return f"name_{name}_{address}_{city}_{state}"
    
    def _school_changed(self, old: Dict, new: Dict) -> bool:
        """Check if a school record has changed."""
        for field in ['name', 'address', 'city', 'state', 'zip_code', 'enrollment']:
            if old.get(field) != new.get(field):
                return True
        return False
    
    def _get_changes(self, old: Dict, new: Dict) -> List[str]:
        """Get list of changed fields."""
        changes = []
        for field in ['name', 'address', 'city', 'state', 'zip_code', 'enrollment']:
            if old.get(field) != new.get(field):
                changes.append({
                    'field': field,
                    'old': old.get(field),
                    'new': new.get(field)
                })
        return changes


class UpdateScheduler:
    """Schedule periodic data updates."""
    
    def __init__(self, config_path: str = None):
        self.config_path = config_path or "output/update_config.json"
        self.last_updates = self._load_last_updates()
    
    def _load_last_updates(self) -> Dict:
        """Load last update timestamps."""
        if Path(self.config_path).exists():
            try:
                with open(self.config_path, 'r') as f:
                    return json.load(f)
            except Exception as e:
                logger.warning(f"Failed to load update config: {e}")
        return {}
    
    def save_last_update(self, source: str, timestamp: datetime = None):
        """Record when a source was last updated."""
        if timestamp is None:
            timestamp = datetime.now()
        
        self.last_updates[source] = timestamp.isoformat()
        
        try:
            with open(self.config_path, 'w') as f:
                json.dump(self.last_updates, f, indent=2)
        except Exception as e:
            logger.warning(f"Failed to save update config: {e}")
    
    def needs_update(self, source: str) -> bool:
        """Check if a source needs updating based on schedule."""
        schedule = UPDATE_SCHEDULES.get(source)
        if not schedule:
            return False
        
        last_update = self.last_updates.get(source)
        now = datetime.now()
        
        if not last_update:
            return True
        
        try:
            last_dt = datetime.fromisoformat(last_update)
        except ValueError:
            return True
        
        if schedule['frequency'] == 'yearly':
            return now.year > last_dt.year
        elif schedule['frequency'] == 'biennial':
            return (now.year % 2) != (last_dt.year % 2)
        elif schedule['frequency'] == 'quarterly':
            return now.month in [1, 4, 7, 10] and now.month > last_dt.month
        elif schedule['frequency'] == 'semester':
            return now.month in [1, 7] and now.month > last_dt.month
        
        return False
    
    def get_update_status(self) -> Dict:
        """Get status of all update schedules."""
        status = {}
        now = datetime.now()
        
        for source, schedule in UPDATE_SCHEDULES.items():
            last_update = self.last_updates.get(source, "Never")
            needs = self.needs_update(source)
            
            status[source] = {
                'schedule': schedule['frequency'],
                'last_update': last_update,
                'needs_update': needs,
                'next_update': self._next_update_time(schedule, now)
            }
        
        return status
    
    def _next_update_time(self, schedule: Dict, now: datetime) -> str:
        """Estimate next update time."""
        if schedule['frequency'] == 'yearly':
            return f"September {now.year + 1}"
        elif schedule['frequency'] == 'biennial':
            next_year = now.year + (2 if now.year % 2 != 0 else 0)
            return f"Even year: {next_year}"
        else:
            return "TBD"


def main():
    """Test change detection."""
    logging.basicConfig(level=logging.INFO)
    
    # Sample baseline and new data
    baseline = [
        {
            'unit_id': '170012345',
            'name': 'Lincoln High School',
            'address': '123 Main St',
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'enrollment': '1200'
        }
    ]
    
    new_data = [
        {
            'unit_id': '170012345',
            'name': 'Lincoln High School',
            'address': '125 Main St',  # Changed address
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'enrollment': '1250'  # Changed enrollment
        },
        {
            'name': 'New School',
            'address': '456 Oak Ave',
            'city': 'Springfield',
            'state': 'IL',
            'zip_code': '62704',
            'enrollment': '500'
        }
    ]
    
    detector = ChangeDetector()
    detector.baseline = baseline
    
    changes = detector.detect_changes(new_data)
    
    print(f"Added: {len(changes['added'])}")
    print(f"Removed: {len(changes['removed'])}")
    print(f"Modified: {len(changes['modified'])}")
    
    for mod in changes['modified']:
        print(f"  {mod['new']['name']} changed:")
        for change in mod['changes']:
            print(f"    {change['field']}: {change['old']} -> {change['new']}")

if __name__ == "__main__":
    main()

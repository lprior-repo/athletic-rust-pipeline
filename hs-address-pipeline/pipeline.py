"""High School Address Pipeline Orchestrator.

Coordinates all components: source fetching, normalization, deduplication,
validation, geocoding, and export.
"""

import logging
import csv
import json
import time
import traceback
from datetime import datetime
from pathlib import Path
from typing import List, Dict

# Import all components
from config import OUTPUT_DIR, BATCH_SIZE
from nccs_crawler import fetch_all_schools as fetch_ccd
from pss_crawler import fetch_all_private_schools as fetch_pss
from state_ed_crawler import fetch_all_state_data as fetch_state_ed
from private_associations_crawler import fetch_all_associations as fetch_associations
from address_normalizer import normalize_and_deduplicate
from geocoder import geocode_and_validate
from update_strategy import ChangeDetector, UpdateScheduler

logger = logging.getLogger(__name__)

class HighSchoolAddressPipeline:
    """Orchestrate the complete high school address pipeline."""
    
    def __init__(self, output_dir: str = OUTPUT_DIR):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        
        self.all_schools = []
        self.fetched_counts = {}
        self.errors = []
        
        # Initialize change detector
        self.change_detector = ChangeDetector(
            baseline_path=str(self.output_dir / "baseline.json")
        )
        self.change_detector.load_baseline()
        
        # Initialize update scheduler
        self.scheduler = UpdateScheduler(
            config_path=str(self.output_dir / "update_config.json")
        )
    
    def run(self, sources: List[str] = None, 
            use_usps: bool = True, 
            use_google: bool = True) -> List[Dict]:
        """Run the complete pipeline.
        
        Args:
            sources: List of sources to fetch from. If None, fetch all.
            use_usps: Use USPS API for validation
            use_google: Use Google Maps for geocoding
        
        Returns:
            List of processed school records
        """
        if sources is None:
            sources = ['ccd', 'pss', 'state_ed', 'associations']
        
        logger.info(f"Starting High School Address Pipeline")
        logger.info(f"Sources: {sources}")
        
        start_time = time.time()
        
        # Step 1: Fetch data from all sources
        self._fetch_phase(sources)
        
        # Step 2: Normalize and deduplicate
        normalized = self._normalize_phase()
        
        # Step 3: Validate and geocode
        validated = self._validate_phase(normalized, use_usps, use_google)
        
        # Step 4: Detect changes
        changes = self._change_detection_phase(validated)
        
        # Step 5: Export results
        self._export_phase(validated, changes)
        
        # Update baseline
        self._update_baseline(validated)
        
        # Update scheduler
        self._update_scheduler(sources)
        
        elapsed = time.time() - start_time
        logger.info(f"Pipeline completed in {elapsed:.2f} seconds")
        logger.info(f"Total schools processed: {len(validated)}")
        
        # Log any errors
        if self.errors:
            logger.warning(f"Pipeline completed with {len(self.errors)} errors")
            for error in self.errors:
                logger.warning(f"  {error}")
        
        return validated
    
    def _fetch_phase(self, sources: List[str]):
        """Fetch data from all configured sources."""
        logger.info("Phase 1: Fetching data from sources")
        
        if 'ccd' in sources:
            logger.info("  Fetching NCES CCD (public schools)...")
            try:
                schools = fetch_ccd()
                self.fetched_counts['ccd'] = len(schools)
                self.all_schools.extend(schools)
                logger.info(f"  Fetched {len(schools)} public schools")
            except Exception as e:
                self.errors.append(f"CCD fetch failed: {e}")
                logger.error(f"  CCD fetch failed: {e}")
        
        if 'pss' in sources:
            logger.info("  Fetching NCES PSS (private schools)...")
            try:
                schools = fetch_pss()
                self.fetched_counts['pss'] = len(schools)
                self.all_schools.extend(schools)
                logger.info(f"  Fetched {len(schools)} private schools")
            except Exception as e:
                self.errors.append(f"PSS fetch failed: {e}")
                logger.error(f"  PSS fetch failed: {e}")
        
        if 'state_ed' in sources:
            logger.info("  Fetching State ED data...")
            try:
                schools = fetch_state_ed()
                self.fetched_counts['state_ed'] = len(schools)
                self.all_schools.extend(schools)
                logger.info(f"  Fetched {len(schools)} state schools")
            except Exception as e:
                self.errors.append(f"State ED fetch failed: {e}")
                logger.error(f"  State ED fetch failed: {e}")
        
        if 'associations' in sources:
            logger.info("  Fetching Private School Association data...")
            try:
                schools = fetch_associations()
                self.fetched_counts['associations'] = len(schools)
                self.all_schools.extend(schools)
                logger.info(f"  Fetched {len(schools)} association schools")
            except Exception as e:
                self.errors.append(f"Associations fetch failed: {e}")
                logger.error(f"  Associations fetch failed: {e}")
        
        logger.info(f"Total schools fetched: {len(self.all_schools)}")
    
    def _normalize_phase(self) -> List[Dict]:
        """Normalize addresses and deduplicate."""
        logger.info("Phase 2: Normalizing and deduplicating")
        
        try:
            normalized = normalize_and_deduplicate(
                self.all_schools, 
                use_usps=False  # Use local normalization for now
            )
            logger.info(f"After normalization: {len(normalized)} schools")
            return normalized
        except Exception as e:
            self.errors.append(f"Normalization failed: {e}")
            logger.error(f"  Normalization failed: {e}")
            return self.all_schools
    
    def _validate_phase(self, schools: List[Dict], 
                        use_usps: bool, 
                        use_google: bool) -> List[Dict]:
        """Validate addresses and geocode."""
        logger.info("Phase 3: Validating and geocoding")
        
        try:
            validated = geocode_and_validate(
                schools,
                use_google=use_google,
                use_usps=use_usps
            )
            logger.info(f"Validation complete for {len(validated)} schools")
            return validated
        except Exception as e:
            self.errors.append(f"Validation failed: {e}")
            logger.error(f"  Validation failed: {e}")
            return schools
    
    def _change_detection_phase(self, schools: List[Dict]) -> Dict:
        """Detect changes from previous update."""
        logger.info("Phase 4: Detecting changes")
        
        if self.change_detector.baseline is None:
            logger.info("  No baseline to compare against")
            return {'added': schools, 'removed': [], 'modified': []}
        
        try:
            changes = self.change_detector.detect_changes(schools)
            logger.info(f"  Added: {len(changes['added'])}")
            logger.info(f"  Removed: {len(changes['removed'])}")
            logger.info(f"  Modified: {len(changes['modified'])}")
            return changes
        except Exception as e:
            self.errors.append(f"Change detection failed: {e}")
            logger.error(f"  Change detection failed: {e}")
            return {'added': schools, 'removed': [], 'modified': []}
    
    def _export_phase(self, schools: List[Dict], changes: Dict):
        """Export results to CSV and JSON."""
        logger.info("Phase 5: Exporting results")
        
        try:
            # CSV export
            csv_path = self.output_dir / "high_school_addresses.csv"
            self._export_csv(schools, csv_path)
            logger.info(f"  Exported CSV to {csv_path}")
            
            # JSON export
            json_path = self.output_dir / "high_school_addresses.json"
            self._export_json(schools, json_path)
            logger.info(f"  Exported JSON to {json_path}")
            
            # Changes report
            changes_path = self.output_dir / "changes.json"
            self._export_changes(changes, changes_path)
            logger.info(f"  Exported changes to {changes_path}")
            
            # Pipeline report
            report_path = self.output_dir / "pipeline_report.json"
            self._export_report(report_path)
            logger.info(f"  Exported report to {report_path}")
            
        except Exception as e:
            self.errors.append(f"Export failed: {e}")
            logger.error(f"  Export failed: {e}")
    
    def _update_baseline(self, schools: List[Dict]):
        """Update the baseline with current data."""
        logger.info("Updating baseline")
        
        try:
            self.change_detector.save_current_as_baseline(schools)
        except Exception as e:
            logger.warning(f"Failed to update baseline: {e}")
    
    def _update_scheduler(self, sources: List[str]):
        """Record update completion in scheduler."""
        for source in sources:
            try:
                self.scheduler.save_last_update(source)
            except Exception as e:
                logger.warning(f"Failed to update scheduler for {source}: {e}")
    
    def _export_csv(self, schools: List[Dict], path: Path):
        """Export schools to CSV file."""
        if not schools:
            return
        
        fieldnames = ['unit_id', 'name', 'address', 'city', 'state', 
                     'zip_code', 'latitude', 'longitude', 'source', 
                     'validated', 'geocode_status']
        
        with open(path, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction='ignore')
            writer.writeheader()
            
            for school in schools:
                # Clean up the record
                clean = {k: v for k, v in school.items() if k in fieldnames}
                writer.writerow(clean)
    
    def _export_json(self, schools: List[Dict], path: Path):
        """Export schools to JSON file."""
        if not schools:
            return
        
        with open(path, 'w') as f:
            json.dump(schools, f, indent=2)
    
    def _export_changes(self, changes: Dict, path: Path):
        """Export change report to JSON."""
        with open(path, 'w') as f:
            json.dump({
                'added': changes['added'],
                'removed': changes['removed'],
                'modified': [
                    {'old': m['old'], 'new': m['new'], 'changes': m['changes']}
                    for m in changes['modified']
                ]
            }, f, indent=2)
    
    def _export_report(self, path: Path):
        """Export pipeline report to JSON."""
        report = {
            'timestamp': datetime.now().isoformat(),
            'fetched_counts': self.fetched_counts,
            'total_fetched': len(self.all_schools),
            'final_count': len([s for s in self.all_schools 
                               if s.get('validated', False)]),
            'errors': self.errors,
            'last_updates': self.scheduler.last_updates
        }
        
        with open(path, 'w') as f:
            json.dump(report, f, indent=2)


def main():
    """Run the high school address pipeline."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
    )
    
    pipeline = HighSchoolAddressPipeline()
    schools = pipeline.run(
        sources=['ccd'],  # Start with just CCD
        use_usps=False,
        use_google=False
    )
    
    print(f"\nPipeline complete. Processed {len(schools)} schools.")
    print(f"Output directory: {pipeline.output_dir}")

if __name__ == "__main__":
    main()

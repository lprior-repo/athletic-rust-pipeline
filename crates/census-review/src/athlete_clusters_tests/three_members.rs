use super::{athlete, cases, grade, school, store_of, verdicts};
use crate::reconcile_athletes;
use census_domain::model::{Gender, ReviewState};

#[test]
fn first_empty_cohort_cannot_hide_contradictory_later_members_of_one_provider_object() {
    for mixed_baseline in [false, true] {
        let schools = [school("School A"), school("School B"), school("School C")];
        let mut rows = schools
            .iter()
            .map(|school| athlete(&school.id, "Jordan Smith", Gender::Boys, "14399169"))
            .collect::<Vec<_>>();
        rows.sort_by(|first, second| first.id.cmp(&second.id));
        grade(&mut rows[1], 11, 2025);
        if mixed_baseline {
            grade(&mut rows[1], 10, 2025);
            grade(&mut rows[2], 11, 2025);
        } else {
            grade(&mut rows[2], 10, 2025);
        }
        let (_dir, store) = store_of(&schools, &rows);
        let report =
            reconcile_athletes(&store, "2026-10-01", false).expect("three-member reconcile");
        assert_eq!(report.decided, 0);
        assert_eq!(report.pending, 1);
        assert!(verdicts(&store).is_empty());
        let filed = cases(&store);
        assert_eq!(filed.len(), 1);
        assert_eq!(filed[0].state, ReviewState::Pending);
        assert_eq!(
            filed[0].member_ids,
            rows.iter().map(|row| row.id.cast()).collect::<Vec<_>>()
        );
        let retained = store
            .snapshot()
            .athletes()
            .expect("retained source evidence");
        assert_eq!(retained.len(), rows.len());
        assert!(rows.iter().all(|row| retained.contains(row)));
        let projection = store
            .athlete_identity_projection()
            .expect("identity projection");
        for row in rows {
            assert_eq!(projection.canonical_id(row.id.as_str()), row.id.as_str());
        }
    }
}

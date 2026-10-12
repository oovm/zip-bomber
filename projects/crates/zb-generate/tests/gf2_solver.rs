use zb_generate::solve_rows;
use zb_types::ErrorCategory;

#[test]
fn solves_unique_2x2() {
    let sol = solve_rows(&[vec![true, true], vec![false, true]], &[true, true]).unwrap();
    assert_eq!(sol.assignment, vec![false, true]);
    assert!(sol.unique);
    assert_eq!(sol.rank, 2);
}

#[test]
fn detects_inconsistency() {
    let err = solve_rows(&[vec![true], vec![true]], &[true, false]).unwrap_err();
    assert_eq!(err.category, ErrorCategory::ConstraintUnsatisfied);
}

#[test]
fn free_vars_forced_zero() {
    let sol = solve_rows(&[vec![true, false]], &[true]).unwrap();
    assert_eq!(sol.assignment, vec![true, false]);
    assert!(!sol.unique);
    assert_eq!(sol.rank, 1);
}

use zb_solver::{
    solve_problem_with_cancel, solve_rows_with_cancel, solve_with_cancel, BitMatrix, CancelToken,
    ConstraintProblem, EquationId, LinearEquation, SolveOutcome, SolverBudget, VariableId,
};
use zb_types::ErrorCategory;

#[test]
fn pre_cancelled_token_rejects_solve_rows() {
    let token = CancelToken::new();
    token.cancel();
    let err = solve_rows_with_cancel(&[vec![true]], &[true], &token).unwrap_err();
    assert_eq!(err.category, ErrorCategory::Cancelled);
    assert_eq!(err.code.as_str(), "ZB_CANCELLED");
}

#[test]
fn cancel_during_elimination_returns_cancelled() {
    // Wide underdetermined system so the pivot loop has many checkpoints.
    let cols = 256usize;
    let mut matrix = BitMatrix::zeros(1, cols);
    matrix.set(0, 0);
    matrix.set(0, cols); // rhs = 1
    let token = CancelToken::new();
    token.cancel();
    let err = solve_with_cancel(matrix, &token).unwrap_err();
    assert_eq!(err.category, ErrorCategory::Cancelled);
}

#[test]
fn solve_problem_maps_cancel_to_outcome() {
    let problem = ConstraintProblem {
        variable_count: 2,
        equations: vec![LinearEquation {
            id: EquationId(0),
            variables: vec![VariableId(0)],
            rhs: true,
        }],
        byte_equations: Vec::new(),
        checksums: Vec::new(),
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    let token = CancelToken::new();
    token.cancel();
    match solve_problem_with_cancel(&problem, &token) {
        SolveOutcome::Cancelled { message } => {
            assert!(message.contains("cancelled"));
        }
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

#[test]
fn fresh_token_still_solves() {
    let sol = solve_rows_with_cancel(
        &[vec![true, true], vec![false, true]],
        &[true, true],
        &CancelToken::new(),
    )
    .unwrap();
    assert_eq!(sol.assignment, vec![false, true]);
}

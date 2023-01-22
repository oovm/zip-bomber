use zb_solver::{
    solve_problem, ConstraintProblem, EquationId, LinearEquation, SolveOutcome, SolverBudget,
    VariableId,
};

fn eq(id: u32, vars: &[u32], rhs: bool) -> LinearEquation {
    LinearEquation {
        id: EquationId(id),
        variables: vars.iter().copied().map(VariableId).collect(),
        rhs,
    }
}

#[test]
fn solves_unique_system_with_evidence() {
    let problem = ConstraintProblem {
        variable_count: 2,
        equations: vec![eq(1, &[0, 1], true), eq(2, &[1], true)],
        budget: SolverBudget::default(),
        problem_digest: Some("demo".into()),
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat {
            assignment,
            unique,
            evidence,
        } => {
            assert_eq!(assignment.bits, vec![false, true]);
            assert!(unique);
            assert_eq!(evidence.rank, 2);
            assert_eq!(evidence.free_bits, 0);
            assert_eq!(evidence.equations_used, 2);
            assert_eq!(evidence.problem_digest.as_deref(), Some("demo"));
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn reports_unsat_on_inconsistent_rows() {
    let problem = ConstraintProblem {
        variable_count: 1,
        equations: vec![eq(1, &[0], true), eq(2, &[0], false)],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Unsat { message, .. } => {
            assert!(!message.is_empty());
        }
        other => panic!("expected Unsat, got {other:?}"),
    }
}

#[test]
fn forces_free_variables_to_zero() {
    let problem = ConstraintProblem {
        variable_count: 2,
        equations: vec![eq(1, &[0], true)],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat {
            assignment,
            unique,
            evidence,
        } => {
            assert_eq!(assignment.bits, vec![true, false]);
            assert!(!unique);
            assert_eq!(evidence.rank, 1);
            assert_eq!(evidence.free_bits, 1);
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn cancels_duplicate_variable_mentions() {
    // x0 XOR x0 XOR x1 = 1  ⇒  x1 = 1
    let problem = ConstraintProblem {
        variable_count: 2,
        equations: vec![eq(1, &[0, 0, 1], true)],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat { assignment, .. } => {
            assert_eq!(assignment.bits, vec![false, true]);
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn rejects_out_of_range_variable() {
    let problem = ConstraintProblem {
        variable_count: 1,
        equations: vec![eq(1, &[3], true)],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::InvalidProblem { message } => {
            assert!(message.contains("outside"));
        }
        other => panic!("expected InvalidProblem, got {other:?}"),
    }
}

#[test]
fn enforces_matrix_bit_budget() {
    let problem = ConstraintProblem {
        variable_count: 8,
        equations: vec![eq(1, &[0], true), eq(2, &[1], false)],
        budget: SolverBudget {
            max_variables: 64,
            max_equations: 64,
            max_matrix_bits: 4,
        },
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::ResourceLimited { message } => {
            assert!(message.contains("max_matrix_bits"));
        }
        other => panic!("expected ResourceLimited, got {other:?}"),
    }
}

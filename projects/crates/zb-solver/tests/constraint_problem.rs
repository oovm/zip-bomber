use zb_solver::{
    crc32, crc_identity_residual_problem, solve_crc_identity_blocks, solve_problem, ByteEquation,
    ByteExpr, ByteTerm, ChecksumAlgorithm, ChecksumConstraint, ConstraintProblem, EquationId,
    LinearEquation, SolveOutcome, SolverBudget, VariableId,
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
        byte_equations: Vec::new(),
        checksums: Vec::new(),
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
        byte_equations: Vec::new(),
        checksums: Vec::new(),
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Unsat {
            message,
            conflicting_equation,
        } => {
            assert!(message.contains("gf2_inconsistent_row="));
            assert_eq!(conflicting_equation, Some(EquationId(2)));
        }
        other => panic!("expected Unsat, got {other:?}"),
    }
}

#[test]
fn forces_free_variables_to_zero() {
    let problem = ConstraintProblem {
        variable_count: 2,
        equations: vec![eq(1, &[0], true)],
        byte_equations: Vec::new(),
        checksums: Vec::new(),
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
        byte_equations: Vec::new(),
        checksums: Vec::new(),
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
        byte_equations: Vec::new(),
        checksums: Vec::new(),
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
fn crc_identity_residual_matches_manual_i_xor_m() {
    // M = 0, so equations are just x_i = b_i for a 32-bit word.
    let columns = vec![0u32; 32];
    let constant = 0xA5A5_5A5A_u32;
    let outcome = solve_crc_identity_blocks(
        &[(constant, columns.as_slice())],
        32,
        SolverBudget::default(),
        Some("crc-id".into()),
    );
    match outcome {
        SolveOutcome::Sat {
            assignment,
            unique,
            evidence,
        } => {
            assert!(unique);
            assert_eq!(evidence.rank, 32);
            let mut value = 0u32;
            for (bit, on) in assignment.bits.iter().enumerate() {
                if *on {
                    value |= 1 << bit;
                }
            }
            assert_eq!(value, constant);
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn crc_identity_rejects_short_column_block() {
    let columns = vec![0u32; 16];
    match crc_identity_residual_problem(
        &[(0, columns.as_slice())],
        32,
        SolverBudget::default(),
        None,
    ) {
        Err(SolveOutcome::InvalidProblem { message }) => {
            assert!(message.contains("expected 32"));
        }
        other => panic!("expected InvalidProblem, got {other:?}"),
    }
}

#[test]
fn enforces_matrix_bit_budget() {
    let problem = ConstraintProblem {
        variable_count: 8,
        equations: vec![eq(1, &[0], true), eq(2, &[1], false)],
        byte_equations: Vec::new(),
        checksums: Vec::new(),
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

fn byte_eq(id: u32, left: ByteTerm, right: ByteTerm) -> ByteEquation {
    ByteEquation {
        id: EquationId(id),
        left,
        right,
    }
}

#[test]
fn reduces_byte_constant_to_bits() {
    let problem = ConstraintProblem {
        variable_count: 8,
        equations: Vec::new(),
        byte_equations: vec![byte_eq(9, ByteTerm::Variable(0), ByteTerm::Constant(0xA5))],
        checksums: Vec::new(),
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat { assignment, unique, .. } => {
            assert!(unique);
            let mut value = 0u8;
            for bit in 0..8 {
                if assignment.bits[bit] {
                    value |= 1 << bit;
                }
            }
            assert_eq!(value, 0xA5);
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn byte_constant_conflict_names_equation() {
    let problem = ConstraintProblem {
        variable_count: 8,
        equations: Vec::new(),
        byte_equations: vec![
            byte_eq(1, ByteTerm::Variable(0), ByteTerm::Constant(1)),
            byte_eq(2, ByteTerm::Variable(0), ByteTerm::Constant(2)),
        ],
        checksums: Vec::new(),
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Unsat {
            conflicting_equation,
            ..
        } => assert_eq!(conflicting_equation, Some(EquationId(2))),
        other => panic!("expected Unsat, got {other:?}"),
    }
}

#[test]
fn byte_copies_share_a_constant() {
    let problem = ConstraintProblem {
        variable_count: 16,
        equations: Vec::new(),
        byte_equations: vec![
            byte_eq(1, ByteTerm::Variable(0), ByteTerm::Variable(1)),
            byte_eq(2, ByteTerm::Variable(1), ByteTerm::Constant(0x3C)),
        ],
        checksums: Vec::new(),
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat { assignment, .. } => {
            for bit in 0..8 {
                assert_eq!(assignment.bits[bit], assignment.bits[8 + bit]);
            }
            let mut value = 0u8;
            for bit in 0..8 {
                if assignment.bits[bit] {
                    value |= 1 << bit;
                }
            }
            assert_eq!(value, 0x3C);
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

fn le_target(crc: u32) -> [ByteExpr; 4] {
    [
        ByteExpr::Constant((crc & 0xff) as u8),
        ByteExpr::Constant(((crc >> 8) & 0xff) as u8),
        ByteExpr::Constant(((crc >> 16) & 0xff) as u8),
        ByteExpr::Constant(((crc >> 24) & 0xff) as u8),
    ]
}

#[test]
fn checksum_accepts_matching_constant_message() {
    let message = b"123456789";
    let problem = ConstraintProblem {
        variable_count: 0,
        equations: Vec::new(),
        byte_equations: Vec::new(),
        checksums: vec![ChecksumConstraint {
            id: EquationId(4),
            algorithm: ChecksumAlgorithm::Crc32Ieee,
            message: message.iter().copied().map(ByteExpr::Constant).collect(),
            target: le_target(crc32(message)),
        }],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    assert!(matches!(solve_problem(&problem), SolveOutcome::Sat { .. }));
}

#[test]
fn checksum_rejects_wrong_constant_and_names_equation() {
    let problem = ConstraintProblem {
        variable_count: 0,
        equations: Vec::new(),
        byte_equations: Vec::new(),
        checksums: vec![ChecksumConstraint {
            id: EquationId(7),
            algorithm: ChecksumAlgorithm::Crc32Ieee,
            message: vec![ByteExpr::Constant(1)],
            target: le_target(0),
        }],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Unsat {
            conflicting_equation,
            ..
        } => assert_eq!(conflicting_equation, Some(EquationId(7))),
        other => panic!("expected Unsat, got {other:?}"),
    }
}

#[test]
fn checksum_solves_one_affine_byte() {
    let problem = ConstraintProblem {
        variable_count: 8,
        equations: Vec::new(),
        byte_equations: Vec::new(),
        checksums: vec![ChecksumConstraint {
            id: EquationId(3),
            algorithm: ChecksumAlgorithm::Crc32Ieee,
            message: vec![ByteExpr::Xor(vec![
                ByteExpr::Constant(0x01),
                ByteExpr::Variable(0),
            ])],
            target: le_target(crc32(&[0x43])),
        }],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::Sat {
            assignment, unique, ..
        } => {
            assert!(unique);
            let mut value = 0u8;
            for bit in 0..8 {
                if assignment.bits[bit] {
                    value |= 1 << bit;
                }
            }
            assert_eq!(value, 0x42);
            assert_eq!(crc32(&[0x01 ^ value]), crc32(&[0x43]));
        }
        other => panic!("expected Sat, got {other:?}"),
    }
}

#[test]
fn deep_xor_nesting_is_rejected_not_spun() {
    let mut expr = ByteExpr::Constant(0);
    for _ in 0..300 {
        expr = ByteExpr::Xor(vec![expr, ByteExpr::Constant(0)]);
    }
    let problem = ConstraintProblem {
        variable_count: 0,
        equations: Vec::new(),
        byte_equations: Vec::new(),
        checksums: vec![ChecksumConstraint {
            id: EquationId(1),
            algorithm: ChecksumAlgorithm::Crc32Ieee,
            message: vec![expr],
            target: le_target(0),
        }],
        budget: SolverBudget::default(),
        problem_digest: None,
    };
    match solve_problem(&problem) {
        SolveOutcome::InvalidProblem { message } => {
            assert!(message.contains("exceeds"));
        }
        other => panic!("expected InvalidProblem, got {other:?}"),
    }
}

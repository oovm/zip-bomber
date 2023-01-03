use zb_generate::{cox_core_program, interpret_lz_quine, is_self_reproducing};

#[test]
fn cox_core_is_self_reproducing() {
    let program = cox_core_program();
    assert!(is_self_reproducing(&program).unwrap());
    assert_eq!(interpret_lz_quine(&program).unwrap(), program);
}

#[test]
fn cox_core_has_expected_length() {
    // 3*L0 + (L4+4) + R4 + (L4+4) + R4 + (L4+4) = 3+5+1+5+1+5 = 20
    assert_eq!(cox_core_program().len(), 20);
}

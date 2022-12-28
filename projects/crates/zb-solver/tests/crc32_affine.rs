use zb_solver::{crc32, crc32_affine_columns, solve_rows, u32_to_bits, CRC32_CHECK};

#[test]
fn ieee_crc32_check_vector() {
    assert_eq!(crc32(b"123456789"), CRC32_CHECK);
}

#[test]
fn affine_recovers_single_unknown_byte() {
    let template = [
        Some(b'1'),
        Some(b'2'),
        None,
        Some(b'4'),
        Some(b'5'),
        Some(b'6'),
        Some(b'7'),
        Some(b'8'),
        Some(b'9'),
    ];
    let target = CRC32_CHECK;
    let (f0, columns) = crc32_affine_columns(&template, target);
    assert_eq!(columns.len(), 8);

    let mut coeffs = vec![vec![false; 8]; 32];
    for (c, col) in columns.iter().enumerate() {
        for r in 0..32 {
            coeffs[r][c] = col[r];
        }
    }
    let rhs = u32_to_bits(f0);
    let sol = solve_rows(&coeffs, &rhs).expect("solvable");
    let mut recovered = 0u8;
    for (bit, on) in sol.assignment.iter().enumerate() {
        if *on {
            recovered |= 1 << bit;
        }
    }
    assert_eq!(recovered, b'3');
}

#[test]
fn crc_is_affine_not_linear() {
    let a = b"aaaaaaaa";
    let b = b"bbbbbbbb";
    let mut axorb = [0u8; 8];
    for i in 0..8 {
        axorb[i] = a[i] ^ b[i];
    }
    assert_ne!(crc32(&axorb), crc32(a) ^ crc32(b));
}

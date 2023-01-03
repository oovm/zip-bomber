use zb_solver::{
    crc32, crc32_affine_columns, crc32_shared_columns, solve_rows, u32_to_bits, CrcByteSlot,
    CRC32_CHECK,
};

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

#[test]
fn shared_columns_match_direct_crc_influence() {
    let bytes = b"0123456789abcdef";
    let slots = [
        CrcByteSlot {
            offset: 2,
            variable: 0,
        },
        CrcByteSlot {
            offset: 8,
            variable: 0,
        },
        CrcByteSlot {
            offset: 12,
            variable: 1,
        },
    ];
    let (constant, columns) = crc32_shared_columns(bytes, &slots, 2).unwrap();
    let mut zeroed = bytes.to_vec();
    zeroed[2] = 0;
    zeroed[8] = 0;
    zeroed[12] = 0;
    assert_eq!(constant, crc32(&zeroed));
    for variable in 0..2 {
        for bit in 0..8 {
            let mut changed = zeroed.clone();
            for slot in slots.iter().filter(|slot| slot.variable == variable) {
                changed[slot.offset] = 1 << bit;
            }
            assert_eq!(columns[variable * 8 + bit], crc32(&changed) ^ constant);
        }
    }
}

#[test]
fn shared_slots_validate_offsets_and_compare_large_affine_assignment() {
    let bytes: Vec<u8> = (0..4097).map(|i| (i % 251) as u8).collect();
    let slots = [
        CrcByteSlot {
            offset: 0,
            variable: 0,
        },
        CrcByteSlot {
            offset: 4096,
            variable: 0,
        },
        CrcByteSlot {
            offset: 258,
            variable: 1,
        },
    ];
    let (constant, columns) = crc32_shared_columns(&bytes, &slots, 2).unwrap();
    let values = [0xabu8, 0x35];
    let mut expected = bytes.clone();
    for slot in slots {
        expected[slot.offset] = values[slot.variable];
    }
    let mut result = constant;
    for (variable, value) in values.iter().enumerate() {
        for bit in 0..8 {
            if value & (1 << bit) != 0 {
                result ^= columns[variable * 8 + bit];
            }
        }
    }
    assert_eq!(result, crc32(&expected));
    assert!(crc32_shared_columns(&bytes, &[slots[0], slots[0]], 2).is_err());
    assert!(crc32_shared_columns(
        &bytes,
        &[CrcByteSlot {
            offset: bytes.len(),
            variable: 0
        }],
        1
    )
    .is_err());
}

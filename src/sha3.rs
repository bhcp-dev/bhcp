const ROUND_CONSTANTS: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

const ROTATIONS: [u32; 25] = [
    0, 1, 62, 28, 27, 36, 44, 6, 55, 20, 3, 10, 43, 25, 39, 41, 45, 15, 21, 8, 18, 2, 61, 56, 14,
];

pub fn digest(input: &[u8]) -> [u8; 64] {
    const RATE: usize = 72;
    let mut state = [0u64; 25];
    let mut chunks = input.chunks_exact(RATE);
    for block in &mut chunks {
        absorb_block(&mut state, block);
        permute(&mut state);
    }
    let remainder = chunks.remainder();
    let mut final_block = [0u8; RATE];
    final_block[..remainder.len()].copy_from_slice(remainder);
    final_block[remainder.len()] = 0x06;
    final_block[RATE - 1] |= 0x80;
    absorb_block(&mut state, &final_block);
    permute(&mut state);
    let mut output = [0u8; 64];
    for (chunk, lane) in output.chunks_exact_mut(8).zip(state) {
        chunk.copy_from_slice(&lane.to_le_bytes());
    }
    output
}

fn absorb_block(state: &mut [u64; 25], block: &[u8]) {
    for (lane, bytes) in state.iter_mut().zip(block.chunks_exact(8)) {
        *lane ^= u64::from_le_bytes(bytes.try_into().unwrap());
    }
}

fn permute(state: &mut [u64; 25]) {
    for constant in ROUND_CONSTANTS {
        let mut columns = [0u64; 5];
        for x in 0..5 {
            columns[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        let mut deltas = [0u64; 5];
        for x in 0..5 {
            deltas[x] = columns[(x + 4) % 5] ^ columns[(x + 1) % 5].rotate_left(1);
        }
        for y in 0..5 {
            for x in 0..5 {
                state[x + 5 * y] ^= deltas[x];
            }
        }

        let mut moved = [0u64; 25];
        for y in 0..5 {
            for x in 0..5 {
                moved[y + 5 * ((2 * x + 3 * y) % 5)] =
                    state[x + 5 * y].rotate_left(ROTATIONS[x + 5 * y]);
            }
        }
        for y in 0..5 {
            for x in 0..5 {
                state[x + 5 * y] =
                    moved[x + 5 * y] ^ ((!moved[(x + 1) % 5 + 5 * y]) & moved[(x + 2) % 5 + 5 * y]);
            }
        }
        state[0] ^= constant;
    }
}

#[cfg(test)]
mod tests {
    use super::digest;

    #[test]
    fn known_empty_vector() {
        assert_eq!(
            hex(&digest(b"")),
            "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26"
        );
        assert_eq!(
            hex(&digest(b"abc")),
            "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0"
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

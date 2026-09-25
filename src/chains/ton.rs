use ed25519_dalek::SigningKey;
use rand::RngCore;
use sha2::{Digest, Sha256};
use crate::chains::evm::hex;

// Official TON Wallet v4R2 Code Cell Representation Hash (constant)
pub const V4R2_CODE_HASH: [u8; 32] = [
    0xfe, 0xb5, 0xff, 0x68, 0x20, 0xe2, 0xff, 0x0d,
    0x94, 0x83, 0xe7, 0xe0, 0xd6, 0x2c, 0x81, 0x7d,
    0x84, 0x67, 0x89, 0xfb, 0x4a, 0xe5, 0x80, 0xc8,
    0x78, 0x86, 0x6d, 0x95, 0x9d, 0xab, 0xd5, 0xc0,
];

pub struct TonGenerator;

impl TonGenerator {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn generate<R: RngCore>(&self, rng: &mut R) -> (String, String) {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let pubkey_bytes = verifying_key.as_bytes();

        let address = Self::derive_v4r2_address(pubkey_bytes, true);
        let private_key = hex::encode(&seed);

        (address, private_key)
    }

    #[inline(always)]
    pub fn derive_v4r2_address(pubkey: &[u8; 32], bounceable: bool) -> String {
        // 1. Data cell representation:
        // d1 (0x00) + d2 (0x51) + seqno (4 bytes: 0) + subwallet_id (4 bytes: 0x29a9a317) + pubkey (32 bytes) + padding (0x40)
        let mut data_repr = [0u8; 43];
        data_repr[0] = 0x00;
        data_repr[1] = 0x51;
        // data_repr[2..6] = seqno = 0
        data_repr[6] = 0x29;
        data_repr[7] = 0xa9;
        data_repr[8] = 0xa3;
        data_repr[9] = 0x17;
        data_repr[10..42].copy_from_slice(pubkey);
        data_repr[42] = 0x40;

        let mut hasher1 = Sha256::new();
        hasher1.update(&data_repr);
        let data_hash = hasher1.finalize();

        // 2. StateInit cell representation:
        // d1 (0x02) + d2 (0x01) + data (0x34) + code_depth (0x0007) + data_depth (0x0000) + code_hash (32) + data_hash (32)
        let mut state_init_repr = [0u8; 71];
        state_init_repr[0] = 0x02;
        state_init_repr[1] = 0x01;
        state_init_repr[2] = 0x34;
        state_init_repr[3] = 0x00;
        state_init_repr[4] = 0x07;
        state_init_repr[5] = 0x00;
        state_init_repr[6] = 0x00;
        state_init_repr[7..39].copy_from_slice(&V4R2_CODE_HASH);
        state_init_repr[39..71].copy_from_slice(&data_hash);

        let mut hasher2 = Sha256::new();
        hasher2.update(&state_init_repr);
        let account_id = hasher2.finalize();

        // 3. User-friendly address (36 bytes):
        // flag (0x11 bounceable EQ, 0x51 non-bounceable UQ) + workchain (0x00) + account_id (32 bytes) + CRC16-CCITT (2 bytes)
        let mut raw_addr = [0u8; 36];
        raw_addr[0] = if bounceable { 0x11 } else { 0x51 };
        raw_addr[1] = 0x00;
        raw_addr[2..34].copy_from_slice(&account_id);

        let crc = Self::crc16_ccitt(&raw_addr[0..34]);
        raw_addr[34] = (crc >> 8) as u8;
        raw_addr[35] = (crc & 0xff) as u8;

        base64_url_encode(&raw_addr)
    }

    #[inline(always)]
    pub fn crc16_ccitt(data: &[u8]) -> u16 {
        let mut crc: u16 = 0;
        for &b in data {
            crc ^= (b as u16) << 8;
            for _ in 0..8 {
                if (crc & 0x8000) != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }
}

#[inline(always)]
pub fn base64_url_encode(data: &[u8; 36]) -> String {
    const URL_SAFE_CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = [0u8; 48];
    let mut di = 0;
    let mut oi = 0;
    while di < 36 {
        let b0 = data[di] as usize;
        let b1 = data[di + 1] as usize;
        let b2 = data[di + 2] as usize;
        out[oi]     = URL_SAFE_CHARSET[(b0 >> 2) & 0x3f];
        out[oi + 1] = URL_SAFE_CHARSET[((b0 << 4) | (b1 >> 4)) & 0x3f];
        out[oi + 2] = URL_SAFE_CHARSET[((b1 << 2) | (b2 >> 6)) & 0x3f];
        out[oi + 3] = URL_SAFE_CHARSET[b2 & 0x3f];
        di += 3;
        oi += 4;
    }
    // SAFETY: URL_SAFE_CHARSET contains only valid ASCII bytes
    unsafe { String::from_utf8_unchecked(out.to_vec()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ton_v4r2_reference_vector() {
        let seed = [1u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let pubkey_bytes = verifying_key.as_bytes();

        let bounceable = TonGenerator::derive_v4r2_address(pubkey_bytes, true);
        assert_eq!(bounceable, "EQDvr_S6wiD4iy6Y6x2c_8yjv-O2bs4xp9bFiQ0w39evpduQ");

        let non_bounceable = TonGenerator::derive_v4r2_address(pubkey_bytes, false);
        assert_eq!(non_bounceable, "UQDvr_S6wiD4iy6Y6x2c_8yjv-O2bs4xp9bFiQ0w39evpYZV");
    }
}

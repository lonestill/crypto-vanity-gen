use std::ffi::CStr;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MetalMatchRecord {
    pub salt_low: u64,
    pub salt_high: u64,
    pub address: [u8; 20],
    pub score: u32,
    pub tier: u32,
    pub pattern_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MetalPointAff {
    pub x: [u32; 8],
    pub y: [u32; 8],
}

extern "C" {
    fn metal_bridge_is_supported() -> i32;
    fn metal_bridge_get_device_name() -> *const c_char;
    fn metal_bridge_init_engine(msl_source: *const c_char) -> i32;
    fn metal_bridge_init_tables(
        table1: *const MetalPointAff,
        table2: *const MetalPointAff,
        table3: *const MetalPointAff,
    ) -> i32;
    fn metal_bridge_run_create2_batch(
        factory: *const u8,
        init_hash: *const u8,
        base_salt: u64,
        total_threads: u32,
        iters_per_thread: u32,
        min_zeros: u32,
        target_nibbles: *const u8,
        target_len_nibbles: u32,
        out_matches: *mut MetalMatchRecord,
        max_matches: u32,
        out_match_count: *mut u32,
    ) -> i32;
    fn metal_bridge_run_eoa_batch(
        base_point: *const MetalPointAff,
        total_threads: u32,
        min_zeros: u32,
        target_nibbles: *const u8,
        target_len_nibbles: u32,
        out_matches: *mut MetalMatchRecord,
        max_matches: u32,
        out_match_count: *mut u32,
    ) -> i32;
    fn metal_bridge_release();
}

pub fn point_from_pubkey(pk: &secp256k1::PublicKey) -> MetalPointAff {
    let uncompressed = pk.serialize_uncompressed();
    let mut x = [0u32; 8];
    let mut y = [0u32; 8];
    for i in 0..8 {
        let x_bytes: [u8; 4] = uncompressed[29 - 4 * i..33 - 4 * i].try_into().unwrap();
        x[i] = u32::from_be_bytes(x_bytes);
        let y_bytes: [u8; 4] = uncompressed[61 - 4 * i..65 - 4 * i].try_into().unwrap();
        y[i] = u32::from_be_bytes(y_bytes);
    }
    MetalPointAff { x, y }
}

fn generate_tables() -> (Vec<MetalPointAff>, Vec<MetalPointAff>, Vec<MetalPointAff>) {
    let secp = secp256k1::Secp256k1::new();
    let mut t1 = Vec::with_capacity(256);
    let mut t2 = Vec::with_capacity(256);
    let mut t3 = Vec::with_capacity(256);

    for a in 0..256u64 {
        let mut priv_bytes = [0u8; 32];
        priv_bytes[24..32].copy_from_slice(&(a + 1).to_be_bytes());
        let sk = secp256k1::SecretKey::from_slice(&priv_bytes).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        t1.push(point_from_pubkey(&pk));
    }

    for b in 0..256u64 {
        let mut priv_bytes = [0u8; 32];
        priv_bytes[24..32].copy_from_slice(&((b + 1) * 256).to_be_bytes());
        let sk = secp256k1::SecretKey::from_slice(&priv_bytes).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        t2.push(point_from_pubkey(&pk));
    }

    for c in 0..256u64 {
        let mut priv_bytes = [0u8; 32];
        priv_bytes[24..32].copy_from_slice(&((c + 1) * 65536).to_be_bytes());
        let sk = secp256k1::SecretKey::from_slice(&priv_bytes).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        t3.push(point_from_pubkey(&pk));
    }

    (t1, t2, t3)
}

pub struct MetalEngine {
    device_name: String,
}

impl MetalEngine {
    pub fn is_supported() -> bool {
        unsafe { metal_bridge_is_supported() == 1 }
    }

    pub fn device_name() -> String {
        unsafe {
            let ptr = metal_bridge_get_device_name();
            if !ptr.is_null() {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            } else {
                "Unknown Metal Device".to_string()
            }
        }
    }

    pub fn init() -> Option<Self> {
        if !Self::is_supported() {
            return None;
        }

        let msl_source = include_str!("../../metal/vanity_engine.metal");
        let c_source = std::ffi::CString::new(msl_source).ok()?;

        let ret = unsafe { metal_bridge_init_engine(c_source.as_ptr()) };
        if ret != 0 {
            return None;
        }

        let (t1, t2, t3) = generate_tables();
        let t_ret = unsafe { metal_bridge_init_tables(t1.as_ptr(), t2.as_ptr(), t3.as_ptr()) };
        if t_ret != 0 {
            return None;
        }

        Some(Self {
            device_name: Self::device_name(),
        })
    }

    pub fn run_create2_batch(
        &self,
        factory: &[u8; 20],
        init_hash: &[u8; 32],
        base_salt: u64,
        total_threads: u32,
        iters_per_thread: u32,
        min_zeros: u32,
        target_nibbles: &[u8],
    ) -> Vec<MetalMatchRecord> {
        let max_matches = 256u32;
        let mut out_matches = vec![
            MetalMatchRecord {
                salt_low: 0,
                salt_high: 0,
                address: [0u8; 20],
                score: 0,
                tier: 0,
                pattern_type: 0,
            };
            max_matches as usize
        ];
        let mut out_count = 0u32;

        let dummy = [0u8; 1];
        let (nibbles_ptr, nibbles_len) = if target_nibbles.is_empty() {
            (dummy.as_ptr(), 0u32)
        } else {
            (target_nibbles.as_ptr(), target_nibbles.len() as u32)
        };

        let ret = unsafe {
            metal_bridge_run_create2_batch(
                factory.as_ptr(),
                init_hash.as_ptr(),
                base_salt,
                total_threads,
                iters_per_thread,
                min_zeros,
                nibbles_ptr,
                nibbles_len,
                out_matches.as_mut_ptr(),
                max_matches,
                &mut out_count as *mut u32,
            )
        };

        if ret == 0 && out_count > 0 {
            out_matches.truncate(out_count as usize);
            out_matches
        } else {
            Vec::new()
        }
    }

    pub fn run_eoa_batch(
        &self,
        base_point: &MetalPointAff,
        total_threads: u32,
        min_zeros: u32,
        target_nibbles: &[u8],
    ) -> Vec<MetalMatchRecord> {
        let max_matches = 256u32;
        let mut out_matches = vec![
            MetalMatchRecord {
                salt_low: 0,
                salt_high: 0,
                address: [0u8; 20],
                score: 0,
                tier: 0,
                pattern_type: 0,
            };
            max_matches as usize
        ];
        let mut out_count = 0u32;

        let dummy = [0u8; 1];
        let (nibbles_ptr, nibbles_len) = if target_nibbles.is_empty() {
            (dummy.as_ptr(), 0u32)
        } else {
            (target_nibbles.as_ptr(), target_nibbles.len() as u32)
        };

        let ret = unsafe {
            metal_bridge_run_eoa_batch(
                base_point as *const MetalPointAff,
                total_threads,
                min_zeros,
                nibbles_ptr,
                nibbles_len,
                out_matches.as_mut_ptr(),
                max_matches,
                &mut out_count as *mut u32,
            )
        };

        if ret == 0 && out_count > 0 {
            out_matches.truncate(out_count as usize);
            out_matches
        } else {
            Vec::new()
        }
    }

    pub fn name(&self) -> &str {
        &self.device_name
    }
}

impl Drop for MetalEngine {
    fn drop(&mut self) {
        unsafe {
            metal_bridge_release();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_keccak::{Hasher, Keccak};
    use secp256k1::{Secp256k1, SecretKey, PublicKey, Scalar};

    #[test]
    fn test_metal_engine_create2_verification() {
        if !MetalEngine::is_supported() {
            return;
        }
        let engine = MetalEngine::init();
        if engine.is_none() {
            return;
        }
        let engine = engine.unwrap();

        let factory = [0x11u8; 20];
        let init_hash = [0x22u8; 32];
        let base_salt = 42u64;

        let matches = engine.run_create2_batch(&factory, &init_hash, base_salt, 1, 1, 0, &[]);
        assert_eq!(matches.len(), 1);

        let gpu_addr = matches[0].address;

        let mut hasher = Keccak::v256();
        hasher.update(&[0xff]);
        hasher.update(&factory);
        let mut salt_buf = [0u8; 32];
        salt_buf[0..8].copy_from_slice(&base_salt.to_le_bytes());
        hasher.update(&salt_buf);
        hasher.update(&init_hash);
        let mut out = [0u8; 32];
        hasher.finalize(&mut out);
        let expected_addr = &out[12..32];

        assert_eq!(gpu_addr, expected_addr);
    }

    #[test]
    fn test_metal_engine_eoa_verification() {
        if !MetalEngine::is_supported() {
            return;
        }
        let engine = MetalEngine::init();
        if engine.is_none() {
            return;
        }
        let engine = engine.unwrap();

        let secp = Secp256k1::new();
        let base_priv = [
            0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81,
            0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0xe7, 0xf8, 0x09,
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        ];
        let base_sk = SecretKey::from_slice(&base_priv).unwrap();
        let base_pk = PublicKey::from_secret_key(&secp, &base_sk);
        let base_point = point_from_pubkey(&base_pk);

        let matches = engine.run_eoa_batch(&base_point, 1, 0, &[]);
        assert_eq!(matches.len(), 4);

        for m in matches {
            let offset = m.salt_low;
            let mut tweak = [0u8; 32];
            tweak[24..32].copy_from_slice(&offset.to_be_bytes());
            let scalar = Scalar::from_be_bytes(tweak).unwrap();
            let sk_match = base_sk.add_tweak(&scalar).unwrap();
            let pk_match = PublicKey::from_secret_key(&secp, &sk_match);
            let uncompressed = pk_match.serialize_uncompressed();

            let mut keccak = Keccak::v256();
            keccak.update(&uncompressed[1..65]);
            let mut hash = [0u8; 32];
            keccak.finalize(&mut hash);
            let expected_addr = &hash[12..32];

            assert_eq!(&m.address, expected_addr);
        }
    }
}

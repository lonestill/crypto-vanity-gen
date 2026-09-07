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

extern "C" {
    fn metal_bridge_is_supported() -> i32;
    fn metal_bridge_get_device_name() -> *const c_char;
    fn metal_bridge_init_engine(msl_source: *const c_char) -> i32;
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
    fn metal_bridge_release();
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
        if ret == 0 {
            Some(Self {
                device_name: Self::device_name(),
            })
        } else {
            None
        }
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
}

use secp256k1::{Secp256k1, SecretKey, PublicKey};
use tiny_keccak::{Hasher, Keccak};
use rand::RngCore;

pub struct EvmGenerator {
    secp: Secp256k1<secp256k1::All>,
}

impl EvmGenerator {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    #[inline(always)]
    pub fn generate_raw<R: RngCore>(&self, rng: &mut R) -> ([u8; 20], [u8; 32]) {
        let mut priv_bytes = [0u8; 32];
        rng.fill_bytes(&mut priv_bytes);
        
        let sk = match SecretKey::from_slice(&priv_bytes) {
            Ok(k) => k,
            Err(_) => {
                priv_bytes[0] = 1;
                SecretKey::from_slice(&priv_bytes).unwrap()
            }
        };

        let pk = PublicKey::from_secret_key(&self.secp, &sk);
        let serialized = pk.serialize_uncompressed();

        let mut keccak = Keccak::v256();
        keccak.update(&serialized[1..65]);
        let mut hash = [0u8; 32];
        keccak.finalize(&mut hash);

        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..32]);

        (address, priv_bytes)
    }

    pub fn format_address_checksum(address: &[u8; 20]) -> String {
        let hex_addr = hex::encode(address);
        let mut keccak = Keccak::v256();
        keccak.update(hex_addr.as_bytes());
        let mut hash = [0u8; 32];
        keccak.finalize(&mut hash);
        let hash_hex = hex::encode(&hash);

        let mut checksummed = String::with_capacity(42);
        checksummed.push_str("0x");
        for (i, c) in hex_addr.chars().enumerate() {
            let hash_nibble = u8::from_str_radix(&hash_hex[i..i+1], 16).unwrap_or(0);
            if hash_nibble >= 8 {
                checksummed.push(c.to_ascii_uppercase());
            } else {
                checksummed.push(c.to_ascii_lowercase());
            }
        }
        checksummed
    }
}

pub mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0x0f) as usize] as char);
        }
        s
    }
}

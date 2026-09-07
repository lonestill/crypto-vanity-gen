use secp256k1::{Secp256k1, SecretKey, PublicKey};
use sha2::{Sha256, Digest};
use ripemd::Ripemd160;
use rand::RngCore;

pub struct BitcoinGenerator {
    secp: Secp256k1<secp256k1::All>,
}

impl BitcoinGenerator {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    #[inline(always)]
    pub fn generate<R: RngCore>(&self, rng: &mut R) -> (String, String) {
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
        let compressed_pubkey = pk.serialize();

        let sha_hash = Sha256::digest(&compressed_pubkey);
        let ripemd_hash = Ripemd160::digest(&sha_hash);

        let address = bech32::segwit::encode_v0(bech32::hrp::BC, &ripemd_hash)
            .unwrap_or_else(|_| "bc1qerror".to_string());

        let mut wif_payload = [0u8; 38];
        wif_payload[0] = 0x80;
        wif_payload[1..33].copy_from_slice(&priv_bytes);
        wif_payload[33] = 0x01;

        let check1 = Sha256::digest(&wif_payload[..34]);
        let check2 = Sha256::digest(&check1);
        wif_payload[34..38].copy_from_slice(&check2[..4]);

        let private_key = bs58::encode(&wif_payload).into_string();

        (address, private_key)
    }
}

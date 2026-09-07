use ed25519_dalek::SigningKey;
use rand::RngCore;

pub struct SolanaGenerator;

impl SolanaGenerator {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    pub fn generate<R: RngCore>(&self, rng: &mut R) -> (String, String) {
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);

        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();

        let address = bs58::encode(verifying_key.as_bytes()).into_string();

        let mut keypair_bytes = [0u8; 64];
        keypair_bytes[..32].copy_from_slice(&seed);
        keypair_bytes[32..].copy_from_slice(verifying_key.as_bytes());
        let private_key = bs58::encode(&keypair_bytes).into_string();

        (address, private_key)
    }
}

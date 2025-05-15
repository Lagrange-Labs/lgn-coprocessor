//! JWT authorization logic used in both worker and gateway

use alloy::primitives::eip191_hash_message;
use alloy::primitives::Signature;
use alloy::signers::k256::ecdsa::RecoveryId;
use alloy::signers::k256::ecdsa::Signature as RecoverableSignature;
use alloy::signers::k256::ecdsa::Signature as K256Signature;
use alloy::signers::k256::ecdsa::SigningKey;
use alloy::signers::k256::ecdsa::VerifyingKey;
use alloy::signers::k256::PublicKey;
use alloy::signers::local::LocalSigner;
use alloy::signers::SignerSync;
use anyhow::Result;
use base64::prelude::BASE64_URL_SAFE_NO_PAD;
use base64::Engine;
use elliptic_curve::consts::U32;
use elliptic_curve::sec1::ToEncodedPoint;
use generic_array::GenericArray;
use jwt::Claims;
use jwt::ToBase64;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JWTAuth {
    claims: Claims,
    #[serde_as(as = "DisplayFromStr")]
    signature: Signature,
}

impl JWTAuth {
    /// Create a new instance and sign with the wallet.
    pub fn new(
        claims: Claims,
        wallet: &LocalSigner<SigningKey>,
    ) -> Result<Self> {
        let msg = claims.to_base64()?;

        let signature = wallet.sign_message_sync(msg.as_bytes())?;

        Ok(Self { claims, signature })
    }

    /// Get the JWT claims.
    pub fn claims(&self) -> &Claims {
        &self.claims
    }

    /// Encode to a Base64 string.
    pub fn encode(&self) -> Result<String> {
        // <https://github.com/mikkyang/rust-jwt/blob/master/src/lib.rs#L164>

        let json_bytes = serde_json::to_vec(&self)?;
        Ok(BASE64_URL_SAFE_NO_PAD.encode(json_bytes))
    }

    /// Decode from a Base64 string.
    pub fn decode(s: &str) -> Result<Self> {
        // <https://github.com/mikkyang/rust-jwt/blob/master/src/lib.rs#L182>

        let json_bytes = BASE64_URL_SAFE_NO_PAD.decode(s)?;
        Ok(serde_json::from_slice(&json_bytes)?)
    }

    /// Recovers the Lagrange public key which was used to sign the claims.
    pub fn recover_public_key(&self) -> Result<String> {
        let msg = self.claims.to_base64()?;
        let message_hash = eip191_hash_message(msg.as_bytes());

        let (recoverable_sig, recovery_id) = self.as_signature()?;
        let verifying_key = VerifyingKey::recover_from_prehash(
            message_hash.as_ref(),
            &recoverable_sig,
            recovery_id,
        )?;

        let public_key = PublicKey::from(&verifying_key);
        let public_key = public_key.to_encoded_point(
            // compress =
            false,
        );
        let public_key = public_key.as_bytes();
        debug_assert_eq!(public_key[0], 0x04);

        let public_key = hex::encode(&public_key[1..]);
        // Must be 64 bytes (128 hex chars).
        debug_assert_eq!(public_key.len(), 128);

        Ok(public_key)
    }

    /// Get the recovery signature.
    /// Copied from ethers-rs since it's private:
    /// <https://github.com/gakonst/ethers-rs/blob/master/ethers-core/src/types/signature.rs#L129>
    fn as_signature(&self) -> Result<(RecoverableSignature, RecoveryId)> {
        let mut recovery_id = self.signature.recid();
        let mut signature = {
            let r_bytes: [_; 32] = self.signature.r().to_be_bytes();
            let s_bytes: [_; 32] = self.signature.s().to_be_bytes();
            let gar: &GenericArray<u8, U32> = GenericArray::from_slice(&r_bytes);
            let gas: &GenericArray<u8, U32> = GenericArray::from_slice(&s_bytes);
            K256Signature::from_scalars(*gar, *gas)?
        };

        // Normalize into "low S" form. See:
        // - https://github.com/RustCrypto/elliptic-curves/issues/988
        // - https://github.com/bluealloy/revm/pull/870
        if let Some(normalized) = signature.normalize_s() {
            signature = normalized;
            recovery_id = RecoveryId::from_byte(recovery_id.to_byte() ^ 1).unwrap();
        }

        Ok((signature, recovery_id))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use elliptic_curve::sec1::Coordinates;
    use jwt::RegisteredClaims;

    use super::*;

    /// Test the JWT authorization process.
    #[test]
    fn test_middleware_jwt_auth_process() -> Result<()> {
        // Create a random wallet.
        let wallet = LocalSigner::random();
        let expected_public_key = get_public_key_by_wallet(&wallet);

        // Encode the JWT auth.
        let auth = JWTAuth::new(test_claims(), &wallet)?;
        let encoded_auth = auth.encode()?;

        // Decode the JWT auth and recover the public key.
        let auth = JWTAuth::decode(&encoded_auth)?;
        let public_key = auth.recover_public_key()?;

        assert_eq!(public_key, expected_public_key);

        Ok(())
    }

    /// Get the public key from wallet.
    fn get_public_key_by_wallet(wallet: &LocalSigner<SigningKey>) -> String {
        let public_key = wallet.credential().verifying_key().to_encoded_point(false);

        // We use another method (different with `recover_public_key`) to get
        // the coordinates of public key, then combine the big-endian bytes.
        let [x, y] = match public_key.coordinates() {
            Coordinates::Uncompressed { x, y } => [x, y],
            _ => unreachable!(),
        };

        let bytes: Vec<_> = x.iter().cloned().chain(*y).collect();

        hex::encode(bytes)
    }

    /// Create test Claims.
    fn test_claims() -> Claims {
        let registered = RegisteredClaims {
            issuer: Some("test-issuer".to_string()),
            subject: Some("test-subject".to_string()),
            issued_at: Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            ..Default::default()
        };

        let private = [(
            "version".to_string(),
            serde_json::Value::String("test-version".to_string()),
        )]
        .into_iter()
        .collect::<BTreeMap<String, serde_json::Value>>();

        Claims {
            registered,
            private,
        }
    }
}

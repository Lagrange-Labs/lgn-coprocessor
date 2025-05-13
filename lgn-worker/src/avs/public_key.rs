use alloy::primitives::U256;
use elliptic_curve::sec1::Coordinates;
use k256::ecdsa::VerifyingKey;

#[derive(Debug)]
pub struct PublicKey {
    pub x: U256,
    pub y: U256,
}

impl PublicKey {
    pub fn to_hex(&self) -> String {
        let xb: [_; 32] = self.x.to_be_bytes();
        let yb: [_; 32] = self.y.to_be_bytes();
        let mut s1 = hex::encode(xb);
        let s2 = hex::encode(yb);
        s1.push_str(&s2);
        s1
    }
}

impl From<&VerifyingKey> for PublicKey {
    fn from(verifying_key: &VerifyingKey) -> Self {
        // Convert to uncompressed point.
        let public_key = verifying_key.to_encoded_point(false);

        let [x, y] = match public_key.coordinates() {
            Coordinates::Uncompressed { x, y } => [x, y],
            _ => unreachable!(),
        };

        // let [x, y] = [x, y].map(|s| U256::from_be_bytes(s));
        //
        // Self { x, y }
        todo!()
    }
}

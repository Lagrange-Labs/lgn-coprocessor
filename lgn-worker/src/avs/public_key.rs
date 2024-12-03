use elliptic_curve::sec1::Coordinates;
use ethers::types::U256;
use k256::ecdsa::VerifyingKey;

#[derive(Debug)]
pub struct PublicKey
{
    pub x: U256,
    pub y: U256,
}

impl PublicKey
{
    pub fn to_hex(&self) -> String
    {
        let mut xb = [0u8; 32];
        self.x
            .to_big_endian(&mut xb[..]);
        let mut yb = [0u8; 32];
        self.y
            .to_big_endian(&mut yb[..]);
        let mut s1 = hex::encode(xb);
        let s2 = hex::encode(yb);
        s1.push_str(&s2);
        s1
    }
}

impl From<&VerifyingKey> for PublicKey
{
    fn from(verifying_key: &VerifyingKey) -> Self
    {
        // Convert to uncompressed point.
        let public_key = verifying_key.to_encoded_point(false);

        let [x, y] = match public_key.coordinates() {
            Coordinates::Uncompressed {
                x,
                y,
            } => {
                [
                    x,
                    y,
                ]
            },
            _ => unreachable!(),
        };

        let [x, y] = [
            x,
            y,
        ]
        .map(|s| U256::from_big_endian(s));

        Self {
            x,
            y,
        }
    }
}

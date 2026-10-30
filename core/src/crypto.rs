use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Noise protocol. XKpsk0 requires 1.5 RTT for handshake.
///
/// XK = Hub static key known; Node sends encrypted static key after DH for hub to verify.
///
/// psk0 = Preshared key is used from first message. Doable because single psk is used.
///
/// XK is chosen over KK so hub does not have to enumerate all keys to verify node's key.
pub const NOISE_PATTERN: &str = "Noise_XKpsk0_25519_ChaChaPoly_BLAKE2s";
/// Length of keys used by noise protocol.
pub const KEYLENGTH: usize = 32;

/// Key with base64 encoding and decoding.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Key(pub [u8; KEYLENGTH]);

impl Key {
    /// Returns whether a key exists, or if it's just an all-0 placeholder.
    fn exists(&self) -> bool {
        return !self.0.iter().all(|&x| x == 0);
    }
}

impl From<Vec<u8>> for Key {
    fn from(v: Vec<u8>) -> Self {
        Self(
            v.try_into()
                .expect("vec key should fit in slice; check KEYLENGTH"),
        )
    }
}

impl Into<Vec<u8>> for Key {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl From<[u8; KEYLENGTH]> for Key {
    fn from(v: [u8; KEYLENGTH]) -> Self {
        Self(v)
    }
}

impl Into<[u8; KEYLENGTH]> for Key {
    fn into(self) -> [u8; KEYLENGTH] {
        self.0
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&BASE64.encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        if s.is_empty() {
            return Ok(Self([0u8; KEYLENGTH]));
        }
        BASE64
            .decode(&s)
            .map_err(|_| serde::de::Error::custom(format!("Invalid base64 key {s:?}")))?
            .try_into()
            .map(Self)
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!("Expected {KEYLENGTH} bytes, got {}", v.len()))
            })
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.exists() {
            write!(f, "none")
        } else {
            write!(f, "{:?}", BASE64.encode(self.0))
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64.encode(self.0))
    }
}

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use snow::StatelessTransportState;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use secmon::traits::iosered::IOSerialized;
use secmon::traits::tryclone::TryClone;

/// Noise protocol. XKpsk0 requires 1.5 RTT for handshake.
///
/// XK = Hub static key known; Node sends encrypted static key after DH for hub to verify.
///
/// psk0 = Preshared key is used from first message. Doable because single psk is used.
///
/// XK is chosen over KK so hub does not have to enumerate all keys to verify node's key.
pub const NOISE_PATTERN: &str = "Noise_XKpsk0_25519_ChaChaPoly_BLAKE2s";

/// Length of keys used by noise protocol.
pub const NOISE_KEYLEN: usize = 32;
/// Maximum message length for noise protocol.
pub const NOISE_MAXLEN: usize = 65535;
/// Tag length for noise protocol (with ChaChaPoly).
pub const NOISE_TAGLEN: usize = 16;
/// Maximum handshake message length for XK handshake.
pub const NOISE_HANDSHAKE_MAXLEN: usize = 64;
/// Maximum handshake time.
pub const NOISE_HANDSHAKE_MAXTIME: Duration = Duration::from_secs(5);
/// Plain text data chunk size for noise protocol.
pub const NOISE_CHUNK_SIZE: usize = NOISE_MAXLEN - NOISE_TAGLEN;
/// Maximum number of chunks allowed for each message.
///
/// Note: There is no theoretical limit; 1024 gives max 67,091,456 bytes of plaintext data.
pub const NOISE_MAX_CHUNK: usize = 1024;

/// Helper function to map `snow` error to `std::io` error.
fn noise_err(e: snow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e)
}

/// Wrapped stateless transport state for manual nonce management.
///
/// This is required so that `try_clone` can be implemented on noise.
///
/// \[WARN\] DO NOT REUSE NONCE! Nonce must be properly set after each read/write message!
pub struct NoiseShared {
    /// Noise protocol transport state
    pub state: StatelessTransportState,

    /// Nonce for `write_message`
    pub tx: Mutex<u64>,
    /// Nonce for `read_message`
    pub rx: Mutex<u64>,
}

/// Wrapped stream with optional noise protocol encryption.
///
/// This serves as a drop-in replacement for streams that implement `IOSerialized`.
pub struct NoisyStream<S: IOSerialized> {
    /// Stream that implements `IOSerialized`.
    ///
    /// The trait is required for unencrypted data to be serialized and transmitted.
    pub stream: S,

    /// Noise protocol transport state and nonces. Handshake should be completed upstream.
    ///
    /// If set, then encrypted data is transmitted.
    ///
    /// Otherwise, unencrypted data is transmitted.
    pub noise: Option<Arc<NoiseShared>>,
}

/// Implement `try_clone` for `NoisyStream` to mimic `TcpStream` behavior.
impl<S: IOSerialized + TryClone> NoisyStream<S> {
    pub fn try_clone(&self) -> io::Result<Self> {
        Ok(Self {
            stream: TryClone::try_clone(&self.stream)?,
            noise: self.noise.clone(),
        })
    }
}

/// Implementation of `IOSerialized` for `NoisyStream`.
///
/// Data is chunked to fit in noise protocol messages. Number of chunks is
/// transmitted first, followed by encrypted data chunk by chunk.
impl<S: IOSerialized> IOSerialized for NoisyStream<S> {
    fn write<T: Serialize>(&mut self, data: &T) -> io::Result<()> {
        let Some(noise) = self.noise.as_mut() else {
            return IOSerialized::write(&mut self.stream, data);
        };

        let mut tx = noise.tx.lock().unwrap();

        let mut buf = [0u8; NOISE_MAXLEN];
        let encoded = postcard::to_stdvec(data).expect("data should be serializable");

        let n_chunks = encoded.len().div_ceil(NOISE_CHUNK_SIZE);
        if n_chunks > NOISE_MAX_CHUNK {
            panic!(
                "serialized data should be smaller than {} bytes",
                NOISE_CHUNK_SIZE * NOISE_MAX_CHUNK
            );
        }

        let n = noise
            .state
            .write_message(*tx, &(n_chunks as u32).to_be_bytes(), &mut buf)
            .map_err(noise_err)?;
        *tx += 1;
        self.stream.write(&buf[..n].to_vec())?;

        for chunk in encoded.chunks(NOISE_CHUNK_SIZE) {
            let n = noise
                .state
                .write_message(*tx, chunk, &mut buf)
                .map_err(noise_err)?;
            *tx += 1;
            self.stream.write(&buf[..n].to_vec())?;
        }

        Ok(())
    }

    fn read<T: DeserializeOwned>(&mut self) -> io::Result<T> {
        self.read_upto(NOISE_MAX_CHUNK * NOISE_CHUNK_SIZE)
    }

    fn read_upto<T: DeserializeOwned>(&mut self, limit: usize) -> io::Result<T> {
        let Some(noise) = self.noise.as_mut() else {
            return IOSerialized::read::<T>(&mut self.stream);
        };

        let mut rx = noise.rx.lock().unwrap();

        let mut buf_pt = [0u8; NOISE_MAXLEN];
        let mut encoded: Vec<u8> = Vec::new();

        let buf = self.stream.read_upto::<Vec<u8>>(NOISE_MAXLEN)?;
        let n = noise
            .state
            .read_message(*rx, &buf.as_slice(), &mut buf_pt)
            .map_err(noise_err)?;
        *rx += 1;
        let n_chunks = u32::from_be_bytes(
            buf_pt[..n]
                .try_into()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        ) as usize;
        if n_chunks > NOISE_MAX_CHUNK {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                format!("data contains {n_chunks} chunks, but limit is {NOISE_MAX_CHUNK} chunks"),
            ));
        }
        if n_chunks * NOISE_CHUNK_SIZE > limit {
            // note: when calling `read_upto` directly,
            // limit should be a tigher bound than `NOISE_MAX_CHUNK * NOISE_CHUNK_SIZE`
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                format!(
                    "data is of {} bytes, but limit is {} bytes",
                    n_chunks * NOISE_CHUNK_SIZE,
                    limit
                ),
            ));
        }

        for _ in 1..=n_chunks {
            let buf = self.stream.read_upto::<Vec<u8>>(NOISE_MAXLEN)?;
            let n = noise
                .state
                .read_message(*rx, &buf.as_slice(), &mut buf_pt)
                .map_err(noise_err)?;
            *rx += 1;
            encoded.extend_from_slice(&buf_pt[..n]);
        }

        let decoded = postcard::from_bytes(&encoded)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(decoded)
    }
}

/// Key with base64 encoding and decoding.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Default)]
pub struct Key(pub [u8; NOISE_KEYLEN]);

impl Key {
    /// Returns whether a key exists, or if it's just an all-0 placeholder.
    pub fn exists(&self) -> bool {
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

impl From<[u8; NOISE_KEYLEN]> for Key {
    fn from(v: [u8; NOISE_KEYLEN]) -> Self {
        Self(v)
    }
}

impl Into<[u8; NOISE_KEYLEN]> for Key {
    fn into(self) -> [u8; NOISE_KEYLEN] {
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
            return Ok(Self([0u8; NOISE_KEYLEN]));
        }
        BASE64
            .decode(&s)
            .map_err(|_| serde::de::Error::custom(format!("Invalid base64 key {s:?}")))?
            .try_into()
            .map(Self)
            .map_err(|v: Vec<u8>| {
                serde::de::Error::custom(format!("Expected {NOISE_KEYLEN} bytes, got {}", v.len()))
            })
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.exists() {
            write!(f, "none")
        } else {
            write!(f, "redacted")
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", BASE64.encode(self.0))
    }
}

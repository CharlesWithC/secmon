use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Result, Write};

/// Trait for serializing and deserializing data.
pub trait IOSerialized {
    /// Serialize `data` and write serialized buffer.
    fn write<T: Serialize>(&mut self, data: &T) -> Result<()>;
    /// Read buffer and return deserialized data.
    fn read<T: DeserializeOwned>(&mut self) -> Result<T>;
    /// Read buffer of up to `limit` bytes and return deserialized data.
    ///
    /// Return an error if buffer is over size limit.
    fn read_upto<T: DeserializeOwned>(&mut self, limit: usize) -> Result<T>;
}

/// Implementation of `IOSerialized` for structs that implement `Read` and `Write`.
///
/// The length of serialized data is written before the serialized data.
///
/// The serialized data must be smaller than 4,294,967,296 bytes.
impl<S: Read + Write> IOSerialized for S {
    fn write<T: Serialize>(&mut self, data: &T) -> Result<()> {
        let encoded = postcard::to_stdvec(data).expect("data should be serializable");
        if u32::try_from(encoded.len()).is_err() {
            panic!("serialized data should be smaller than {} bytes", u32::MAX);
        }
        self.write_all(&(encoded.len() as u32).to_be_bytes())?;
        self.write_all(&encoded)
    }

    fn read<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.read_upto(u32::MAX as usize)
    }

    fn read_upto<T: DeserializeOwned>(&mut self, limit: usize) -> Result<T> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                format!("data is of {len} bytes, but limit is {limit} bytes"),
            ));
        }

        let mut data = vec![0u8; len];
        self.read_exact(&mut data)?;

        let decoded = postcard::from_bytes(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(decoded)
    }
}

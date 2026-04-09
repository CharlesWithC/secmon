use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Result, Write};
use std::net::TcpStream;

/// Trait for serializing and deserializing data on streams.
pub trait IOSerialized {
    /// Serialize `data` and write serialized data.
    fn write<T: Serialize>(&mut self, data: &T) -> Result<()>;
    /// Read data and return deserialized data.
    fn read<T: DeserializeOwned>(&mut self) -> Result<T>;
}

/// Implementation of `IOSerialized` for `TcpStream`.
/// 
/// The length of serialized data is transmitted before the serialized data.
/// 
/// The serialized data must be smaller than 4,294,967,296 bytes.
impl IOSerialized for TcpStream {
    /// Serialize `data` and write serialized data to `TcpStream`.
    fn write<T: Serialize>(&mut self, data: &T) -> Result<()> {
        let encoded = postcard::to_stdvec(data).unwrap();
        self.write_all(&(encoded.len() as u32).to_be_bytes())?;
        self.write_all(&encoded)
    }

    /// Read data from `TcpStream` and return deserialized data.
    fn read<T: DeserializeOwned>(&mut self) -> Result<T> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.read_exact(&mut data)?;

        let decoded = postcard::from_bytes(&data).unwrap();
        Ok(decoded)
    }
}

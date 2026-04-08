use serde::{Serialize, de::DeserializeOwned};
use std::io::{Read, Result, Write};
use std::net::TcpStream;

pub trait SendRecv {
    fn send<T: Serialize>(&mut self, data: &T) -> Result<()>;
    fn recv<T: DeserializeOwned>(&mut self) -> Result<T>;
}

impl SendRecv for TcpStream {
    fn send<T: Serialize>(&mut self, data: &T) -> Result<()> {
        let encoded = postcard::to_stdvec(data).unwrap();
        self.write_all(&(encoded.len() as u32).to_be_bytes())?;
        self.write_all(&encoded)
    }

    fn recv<T: DeserializeOwned>(&mut self) -> Result<T> {
        let mut len_buf = [0u8; 4];
        self.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut data = vec![0u8; len];
        self.read_exact(&mut data)?;

        let decoded = postcard::from_bytes(&data).unwrap();
        Ok(decoded)
    }
}

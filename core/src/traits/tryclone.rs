use std::io;
use std::net::TcpStream;

/// Helper trait to bound structs and allow `try_clone` calls on generics.
pub trait TryClone: Sized {
    fn try_clone(&self) -> io::Result<Self>;
}

/// Implementation of `TryClone` for `TcpStream`.
impl TryClone for TcpStream {
    fn try_clone(&self) -> io::Result<Self> {
        TcpStream::try_clone(self)
    }
}

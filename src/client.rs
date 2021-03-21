use crate::pk;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::net::TcpStream;

/// Represents a connection to a server.
pub struct Client<M>
{
    /// Type of the communication messages.
    msg_type: PhantomData<M>,
    /// Tcp stream to the server.
    stream: TcpStream,
}

impl<M> Client<M>
where
    M: Serialize + DeserializeOwned + Debug
{
    /// Create a new client by connecting to a server by its address.
    pub fn connect(addr: &str) -> Result<Self, Box<dyn Error>> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self {
            msg_type: PhantomData,
            stream
        })
    }

    /// Send a message to the server.
    pub fn send(&mut self, msg: M) -> Result<(), Box<dyn Error>> {
        pk::send(msg, &mut self.stream)?;
        Ok(())
    }

    /// Receive a message from the server (blocks).
    pub fn recv(&mut self) -> Result<M, Box<dyn Error>> {
        let msg = pk::recv(&mut self.stream)?;
        Ok(msg)
    }
}
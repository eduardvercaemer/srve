use std::net::TcpStream;
use bincode::{serialize, deserialize};
use std::io::{Write, Read, ErrorKind};
use std::error::Error;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Send a message via a tcp stream (blocking).
pub fn send<M>(msg: M, stream: &mut TcpStream) -> Result<(), Box<dyn Error>>
where
    M: Serialize
{
    // make sure we can block (necessary?).
    stream.set_nonblocking(false)?;

    // attempt serialization of the message
    let data = serialize(&msg)?;
    let len = serialize(&(data.len() as u64))?;

    // attempt to write to stream
    stream.write_all(len.as_slice())?;
    stream.write_all(data.as_slice())?;

    Ok(())
}

/// Attempts to receive a message from the tcp stream (blocking).
pub fn recv<M>(stream: &mut TcpStream) -> Result<M, Box<dyn Error>>
where
    M: DeserializeOwned
{
    // we want to block
    stream.set_nonblocking(false)?;

    // get length first
    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf[..])?;
    let len = deserialize::<u64>(&buf[..])? as usize;

    // then get the message
    let mut buf = Vec::new();
    buf.resize(len, 0u8);
    stream.read_exact(buf.as_mut_slice())?;
    Ok(deserialize(buf.as_slice())?)
}

/// Return immediately if there are no incoming messages, otherwise will block until we can
/// receive a complete message.
pub fn try_recv<M>(stream: &mut TcpStream) -> Result<Option<M>, Box<dyn Error>>
where
    M: DeserializeOwned
{
    // we do not want to block
    stream.set_nonblocking(true)?;

    // peek a byte
    // TODO: possible improvement here ?
    let mut buf = [0u8; 1];
    match stream.peek(&mut buf[..]) {
        Ok(size) => {
            if size == 0 { // empty
                return Ok(None);
            }
        }
        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            // empty
            return Ok(None);
        }
        Err(e) => return Err(Box::new(e)),
    }

    // now that we can expect a message, block
    stream.set_nonblocking(false)?;

    // get length first
    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf[..])?;
    let len = deserialize::<u64>(&buf[..])? as usize;

    // then get the message
    let mut buf = Vec::new();
    buf.resize(len, 0u8);
    stream.read_exact(buf.as_mut_slice())?;
    Ok(Some(deserialize(buf.as_slice())?))
}

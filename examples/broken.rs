//! Simulate a broken client.
extern crate text_io;

use std::net::{TcpStream, Shutdown};
use std::io::{Write, Stdout, stdout};
use std::error::Error;
use text_io::read;

/// Stablish a connection and send an incomplete message.
/// Closes the connection before sending the expected amount of bytes.
fn pk_incomplete() -> Result<(), Box<dyn Error>>{
    let mut s = TcpStream::connect("127.0.0.1:6935")?;

    // server expects 8 bytes for 'len' attribute
    let mut buf0 = Vec::new();
    buf0.push(8u8);
    buf0.resize(8, 0u8);
    s.write_all(buf0.as_slice())?;

    // now the server expects 8 bytes of data, let's send only 5
    let buf1 = [1u8; 5];
    s.write_all(&buf1[..])?;

    // then let's close the connection
    s.shutdown(Shutdown::Both)?;
    Ok(())
}

/// Stablish a connection and send a bad message.
/// Sends a message that will trigger a deserialization error on the server.
fn pk_bad() -> Result<(), Box<dyn Error>>{
    let mut s = TcpStream::connect("127.0.0.1:6935")?;

    // server expects 8 bytes for 'len' attribute
    let mut buf0 = Vec::new();
    buf0.push(8u8);
    buf0.resize(8, 0u8);
    s.write_all(buf0.as_slice())?;

    // now the server expects 8 bytes of data, let's send only 5
    let buf1 = [1u8; 8];
    s.write_all(&buf1[..])?;

    // then let's close the connection
    s.shutdown(Shutdown::Both)?;
    Ok(())
}

/// Allows the user to simulate multiple types of client errors.
fn main() {
    println!(" ...::: COMMANDS :::... ");
    println!();
    println!("> bad");
    println!("> incomplete");
    println!();

    loop {
        write!(stdout().lock(), "> ");

        let s: String = read!();

        match s.as_str() {
            "incomplete" => {
                pk_incomplete().unwrap();
            }
            "bad" => {
                pk_bad().unwrap();
            }
            _ => {}
        }
    }
}
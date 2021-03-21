#[macro_use]
extern crate serde_derive;
extern crate text_io;
extern crate srve;

mod shared;
use text_io::read;
use srve::Client;
use shared::Msg;

fn main() {
    const ADDR: &'static str = "127.0.0.1:6935";

    println!("attempting connection to {}", ADDR);
    let mut client: Client<Msg> = Client::connect(ADDR)
        .expect("Failed to connect");

    loop {
        // send something based on input
        let s: String = read!();
        match s.as_str() {
            "add" => {
                let x: i32 = read!();
                client.send(Msg::Add(x)).unwrap();
                match client.recv() {
                    Err(e) => panic!("err: {}", e),
                    Ok(Msg::Ok) => {
                        println!("server: ok");
                    }
                    _ => {
                        eprintln!("server: unexpected response");
                    }
                }
            }
            "sub" => {
                let x: i32 = read!();
                client.send(Msg::Sub(x)).unwrap();
                match client.recv() {
                    Err(e) => panic!("err: {}", e),
                    Ok(Msg::Ok) => {
                        println!("server: ok");
                    }
                    _ => {
                        eprintln!("server: unexpected response");
                    }
                }
            }
            "print" => {
                client.send(Msg::Print).unwrap();
                match client.recv() {
                    Err(e) => panic!("err: {}", e),
                    Ok(Msg::Value(x)) => {
                        println!("server: {}", x);
                    }
                    _ => {
                        eprintln!("server: unexpected response");
                    }
                }
            }
            _ => {
                println!("valid commands:");
                println!("> add x");
                println!("> sub x");
                println!("> print");
            }
        }
    }
}
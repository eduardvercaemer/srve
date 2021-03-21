#[macro_use]
extern crate serde_derive;
extern crate srve;

mod shared;
use srve::Server;
use shared::{State, Msg};

fn main() {
    const ADDR: &'static str = "127.0.0.1:6935";

    Server::<State,Msg>::bind(ADDR)
        .expect("Failed to bind server")
        // calback function for new connections
        .on_connection(|conn| {
            println!("!! INBOUND :: {}", conn.addr);
        })
        // callback function for new messages
        .on_message(|conn, msg| {
            match msg {
                Msg::Add(x) => {
                    println!("{} :: add {}", conn.addr, x);
                    conn.value += x;
                    conn.send(Msg::Ok).unwrap();
                },
                Msg::Sub(x) => {
                    println!("{} :: sub {}", conn.addr, x);
                    conn.value -= x;
                    conn.send(Msg::Ok).unwrap();
                }
                Msg::Print => {
                    println!("{} :: value = {}", conn.addr, conn.value);
                    conn.send(Msg::Value(conn.value)).unwrap();
                }
                _ => {
                    eprintln!("{} :: Unexpected message", conn.addr);
                    conn.send(Msg::Err).unwrap();
                }
            }
        })
        .on_close(|conn| {
            println!("!! CLOSED :: {}", conn.addr);
        })
        .run();
}
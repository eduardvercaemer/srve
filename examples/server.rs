#[macro_use]
extern crate serde_derive;
extern crate srve;
extern crate simple_logger;
extern crate log;

mod shared;
use srve::Server;
use shared::{State, Msg};
use log::{info, warn};

fn main() {
    const ADDR: &'static str = "127.0.0.1:6935";

    simple_logger::SimpleLogger::new().init().unwrap();

    Server::<State,Msg>::bind(ADDR)
        .expect("Failed to bind server")
        // calback function for new connections
        .on_connection(|conn| {})
        // callback function for new messages
        .on_message(|conn, msg| {
            match msg {
                Msg::Add(x) => {
                    info!("{} :: add {}", conn.addr, x);
                    conn.value += x;
                    conn.send(Msg::Ok).unwrap();
                },
                Msg::Sub(x) => {
                    info!("{} :: sub {}", conn.addr, x);
                    conn.value -= x;
                    conn.send(Msg::Ok).unwrap();
                }
                Msg::Print => {
                    info!("{} :: value = {}", conn.addr, conn.value);
                    conn.send(Msg::Value(conn.value)).unwrap();
                }
                _ => {
                    warn!("{} :: unexpected message", conn.addr);
                    conn.send(Msg::Err).unwrap();
                }
            }
        })
        .on_close(|conn| {})
        .run();
}
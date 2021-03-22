#[macro_use]
extern crate serde_derive;
extern crate srve;
extern crate simple_logger;
extern crate log;

mod shared;
use srve::Server;
use shared::{State, Msg, ADDR};
use log::{info, trace, warn, LevelFilter};

fn main() {
    /* select log level for crate */
    simple_logger::SimpleLogger::new()
        .with_module_level("srve", LevelFilter::Off)
        .init()
        .unwrap();

    Server::<State,Msg>::bind(ADDR)
        .expect("Failed to bind server")
        // calback function for new connections
        .on_connection(|_conn| {
            trace!("connection cb");
        })
        // callback function for new messages
        .on_message(|conn, msg| {
            trace!("message cb");

            match msg {
                Msg::Add(x) => {
                    info!("{} :: add {}", conn.addr, x);
                    conn.value += x;
                    conn.send(Msg::Ok).unwrap_or_else(|_| {
                        warn!("send failed");
                    });
                },
                Msg::Sub(x) => {
                    info!("{} :: sub {}", conn.addr, x);
                    conn.value -= x;
                    conn.send(Msg::Ok).unwrap_or_else(|_| {
                        warn!("send failed");
                    });
                }
                Msg::Print => {
                    info!("{} :: value = {}", conn.addr, conn.value);
                    conn.send(Msg::Value(conn.value)).unwrap_or_else(|_| {
                        warn!("send failed");
                    });
                }
                _ => {
                    warn!("{} :: unexpected message", conn.addr);
                    conn.send(Msg::Err).unwrap_or_else(|_| {
                        warn!("send failed");
                    });
                }
            }
        })
        // callback function for connection closing
        .on_closed(|_conn| {
            info!("closed cb");
        })
        // callback function for unexpected connection closing
        .on_closed_unexpected(|_conn| {
            info!("closed unexpected cb");
        })
        // callback function for unexpected connection errors (e.g. bad msg)
        .on_error(|_conn, _e| {
            info!("error cb");
        })
        // start the server
        .run();
}
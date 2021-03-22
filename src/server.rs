use crate::pk::{self, RecvResult};
use log::{info, warn};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::marker::PhantomData;
use std::net::{TcpListener, SocketAddr, TcpStream, Shutdown};
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use std::thread;

/// Represents our server.
pub struct Server<S,M> {
    /// Receive new connections from the slave thread.
    listener: Receiver<ConnInbound>,
    /// List of current connections.
    conns: Vec<Conn<S,M>>,

    /* connection callbacks */
    cb_connection: Option<fn(&mut Conn<S,M>)>,
    cb_message: Option<fn (&mut Conn<S,M>, M)>,
    cb_close: Option<fn (&mut Conn<S,M>)>,
}

/// Server side representation of a client connection.
pub struct Conn<S,M> {
    /// Tcp stream to client.
    stream: TcpStream,
    /// Type of the messages.
    msg_type: PhantomData<M>,
    /// Connection state.
    state: Box<S>,
    /// Wether the connection has been set as should close.
    close: bool,

    /// Address of client connection.
    pub addr: SocketAddr,
}

/// Represent new inbound connections.
struct ConnInbound {
    stream: TcpStream,
    addr: SocketAddr,
}

impl<S, M> Server<S, M>
where
    S: Default,
    M: Serialize + DeserializeOwned,
{
    /// Create a new server by binding to a listening TCP port.
    pub fn bind(addr: &str) -> Result<Self, Box<dyn Error>> {
        // create slave thread with blocking tcp listener
        let sock = TcpListener::bind(addr)?;
        sock.set_nonblocking(false)?;
        let (tx, rx) = mpsc::channel::<ConnInbound>();
        // TODO: handle the child thread somewhere
        thread::spawn(move || loop {
            // listen for possible new connections
            match sock.accept() {
                Ok((stream, addr)) => { // new connection
                    // TODO: handle channel unwrap
                    tx.send(ConnInbound { stream, addr }).unwrap();
                }
                Err(e) => { // error
                    // TODO: panic for now
                    panic!("err: {}", e);
                }
            }
        });

        // return the new server
        let conns: Vec<Conn<S,M>> = Vec::new();
        Ok(Self {
            listener: rx,
            conns,
            cb_connection: None,
            cb_message: None,
            cb_close: None,
        })
    }

    /// Setup a callback for each new connection.
    pub fn on_connection(mut self, cb: fn (&mut Conn<S,M>)) -> Self {
        self.cb_connection = Some(cb);
        self
    }

    /// Setup a calback for each received message.
    pub fn on_message(mut self, cb: fn (&mut Conn<S,M>, M)) -> Self {
        self.cb_message = Some(cb);
        self
    }

    /// Setup a callback for closed connections.
    pub fn on_close(mut self, cb: fn(&mut Conn<S,M>)) -> Self {
        self.cb_close = Some(cb);
        self
    }

    /// Run the server by using the given callback function on connections.
    pub fn run(mut self) -> ! {
        /* this loop will run forever */
        loop {
            // check for new connections from slave thread
            // TODO: handle try_recv errors
            while let Ok(inbound) = self.listener.try_recv() {
                let mut conn = Conn::new(inbound);
                info!("{} :: inbound", conn.addr);
                if let Some(cb) = self.cb_connection {
                    cb(&mut conn);
                }
                self.conns.push(conn);
            }

            /* handle current list of connections
             * TODO: use thread pool for better performance ?
             */
            for mut conn in self.conns.iter_mut() {
                match conn.try_receive() {
                    /* succesfully received a message */
                    Ok(RecvResult::Some(msg)) => {
                        info!("{} :: recveived message", conn.addr);
                        if let Some(cb) = self.cb_message {
                            cb(&mut conn, msg);
                        }
                    }
                    /* client closed connection */
                    Ok(RecvResult::Closed) => {
                        info!("{} :: closed connection", conn.addr);
                        if let Err(e) = conn.stream.shutdown(Shutdown::Both) {
                            warn!("{} :: error closing stream: {}", conn.addr, e);
                        }
                        if let Some(cb) = self.cb_close {
                            cb(conn);
                        }
                        conn.close = true;
                    }
                    /* client remains silent */
                    Ok(RecvResult::None) => {}
                    /* client closed unexpectedly, terminates connection */
                    Ok(RecvResult::ClosedWrongly) => {
                        warn!("{} :: closed unexepectedly", conn.addr);
                        conn.close = true;
                    }
                    /* any other error, terminates connection as well */
                    Err(e) => {
                        warn!("{} :: unexpected error: {}", conn.addr, e);
                        conn.close = true;
                    }
                }
            }
            self.conns.retain(|conn| {
                !conn.close
            });
        }
    }
}

impl<S,M> Conn<S,M>
where
    S: Default,
    M: Serialize + DeserializeOwned
{
    /// Create a new connection from its tcp stream and socket address.
    /// Its initial states will be generates as per its implementation of the
    /// Default trait.
    fn new(inbound: ConnInbound) -> Self {
        Self {
            stream: inbound.stream,
            addr: inbound.addr,
            msg_type: PhantomData,
            state: Box::new(<S as Default>::default()),
            close: false,
        }
    }

    /// Attempt to receive and decode incoming packets in this connection.
    fn try_receive(&mut self) -> Result<RecvResult<M>, Box<dyn Error>> {
        pk::try_recv(&mut self.stream)
    }

    /// Send a message back.
    pub fn send(&mut self, msg: M) -> Result<(), Box<dyn Error>> {
        pk::send(msg, &mut self.stream)
    }
}

impl<S,M> Deref for Conn<S,M> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &* self.state
    }
}

impl<S,M> DerefMut for Conn<S,M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.state
    }
}
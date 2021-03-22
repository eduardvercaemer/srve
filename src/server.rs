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
    cb_closed: Option<fn (&mut Conn<S,M>)>,
    cb_closed_unexpected: Option<fn (&mut Conn<S,M>)>,
    cb_connection: Option<fn (&mut Conn<S,M>)>,
    cb_error: Option<fn (&mut Conn<S,M>, Box<dyn Error>)>,
    cb_message: Option<fn (&mut Conn<S,M>, M)>,
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
    should_close: bool,

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
            /* connection callbacks */
            cb_closed: None,
            cb_closed_unexpected: None,
            cb_connection: None,
            cb_error: None,
            cb_message: None,
        })
    }

    /// Setup a callback for closed connections.
    ///
    /// This callback will only _ever_ be run, when the client terminates the connection
    /// to the server.
    pub fn on_closed(mut self, cb: fn(&mut Conn<S,M>)) -> Self {
        self.cb_closed = Some(cb);
        self
    }

    /// Setup a callback for unexpectedly closed connections.
    ///
    /// This callback will be run when the connection is closed by the client when the
    /// server is receiving a message.
    pub fn on_closed_unexpected(mut self, cb: fn(&mut Conn<S,M>)) -> Self {
        self.cb_closed_unexpected = Some(cb);
        self
    }

    /// Setup a callback for each new connection.
    ///
    /// This callback will be run when the server stablishes a new connection with a client,
    /// _before_ any messages are recevied.
    pub fn on_connection(mut self, cb: fn (&mut Conn<S,M>)) -> Self {
        self.cb_connection = Some(cb);
        self
    }

    /// Setup a callback for connection errors.
    ///
    /// This callback will be run whenever there is an error attempting to get the next
    /// message from a connection, e.g. a bad message that fails to be deserialized.
    pub fn on_error(mut self, cb: fn(&mut Conn<S,M>, Box<dyn Error>)) -> Self {
        self.cb_error = Some(cb);
        self
    }

    /// Setup a calback for each received message.
    ///
    /// This is the main callbcak, which is run every time a connection sends a new message.
    pub fn on_message(mut self, cb: fn (&mut Conn<S,M>, M)) -> Self {
        self.cb_message = Some(cb);
        self
    }

    /// Run the server by using the given callback function on connections.
    ///
    /// At this moment, the server starts adding inbound connections and handling them,
    /// by using the callbacks given during its creation.
    pub fn run(mut self) -> ! {
        /* this loop will run forever */
        loop {
            // check for new connections from slave thread
            // TODO: handle try_recv errors
            /* we use connection callback here */
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
                /* skip closed connections */
                if conn.should_close {  continue; }

                match conn.try_receive() {
                    /* succesfully received a message */
                    Ok(RecvResult::Some(msg)) => {
                        info!("{} :: message", conn.addr);
                        if let Some(cb) = self.cb_message {
                            cb(&mut conn, msg);
                        }
                    }
                    /* client closed connection */
                    Ok(RecvResult::Closed) => {
                        info!("{} :: closed", conn.addr);
                        attempt_shutdown(&mut conn.stream);
                        if let Some(cb) = self.cb_closed {
                            cb(conn);
                        }
                        conn.should_close = true;
                    }
                    /* client remains silent */
                    /*
                     * TODO: implement some sort of timeout system
                     */
                    Ok(RecvResult::None) => {}
                    /* client closed unexpectedly, terminates connection */
                    Ok(RecvResult::ClosedWrongly) => {
                        warn!("{} :: closed unexepectedly", conn.addr);
                        attempt_shutdown(&mut conn.stream);
                        if let Some(cb) = self.cb_closed_unexpected {
                            cb(conn);
                        }
                        conn.should_close = true;
                    }
                    /* any other error, terminates connection as well */
                    Err(e) => {
                        warn!("{} :: error: {}", conn.addr, e);
                        attempt_shutdown(&mut conn.stream);
                        if let Some(cb) = self.cb_error {
                            cb(conn, e);
                        }
                        conn.should_close = true;
                    }
                }
            }
            self.conns.retain(|conn| {
                !conn.should_close
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
            should_close: false,
        }
    }

    /// Attempt to receive and decode incoming packets in this connection.
    fn try_receive(&mut self) -> Result<RecvResult<M>, Box<dyn Error>> {
        pk::try_recv(&mut self.stream)
    }

    /// Send a message back.
    pub fn send(&mut self, msg: M) -> Result<(), Box<dyn Error>> {
        match pk::send(msg, &mut self.stream) {
            /* we failed to send the message */
            Err(e) => {
                warn!("{} :: err send: {}", self.addr, e);
                Err(e)
            }
            Ok(_) => Ok(()),
        }
    }

    /// Close the connection with the client.
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        attempt_shutdown(&mut self.stream);
        self.should_close = true;
        Ok(())
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

fn attempt_shutdown(stream: &mut TcpStream) {
    if let Err(e) = stream.shutdown(Shutdown::Both) {
        warn!("failed to shutdown stream: {}", e);
    }
}
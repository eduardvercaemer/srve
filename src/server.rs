use crate::pk;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::LinkedList;
use std::error::Error;
use std::marker::PhantomData;
use std::net::{TcpListener, SocketAddr, TcpStream};
use std::ops::{Deref, DerefMut};
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use std::thread;

/// Represents our server.
pub struct Server<S,M> {
    /// Receive new connections from the slave thread.
    listener: Receiver<ConnInbound>,
    /// List of current connections.
    conns: LinkedList<Conn<S,M>>,

    /* connection callbacks */
    cb_connection: Option<fn(&mut Conn<S,M>)>,
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
        let conns: LinkedList<Conn<S,M>> = LinkedList::new();
        Ok(Self {
            listener: rx,
            conns,
            cb_connection: None,
            cb_message: None,
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

    /// Run the server by using the given callback function on connections.
    pub fn run(mut self) -> ! {
        /* this loop will run forever */
        loop {
            // check for new connections from slave thread
            // TODO: handle try_recv errors
            while let Ok(inbound) = self.listener.try_recv() {
                let mut conn = Conn::new(inbound);
                if let Some(cb) = self.cb_connection {
                    cb(&mut conn);
                }
                self.conns.push_back(conn);
            }

            // handle current connections
            for mut conn in self.conns.iter_mut() {
                // see if there are new packets
                // TODO: handle closed connections somehow
                match conn.try_receive() {
                    Ok(Some(msg)) => {
                        if let Some(cb) = self.cb_message {
                            cb(&mut conn, msg);
                        }
                    }
                    Ok(None) => {}
                    // TODO: handle connection errors
                    Err(e) => panic!("err: {}", e),
                }
            }
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
        }
    }

    /// Attempt to receive and decode incoming packets in this connection.
    fn try_receive(&mut self) -> Result<Option<M>, Box<dyn Error>> {
        pk::try_recv(&mut self.stream)
    }

    /// Send a message back.
    pub fn send(&mut self, msg: M) -> Result<(), Box<dyn Error>> {
        pk::send(msg, &mut self.stream)?;
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
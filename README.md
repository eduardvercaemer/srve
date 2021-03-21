# srve

`srve` allows you to create simple network communication in a server client
model, the server holds a bunch of client connections, each with its own state,
and you can communicate with them via messages.

The server uses a background thread to listen for new connections.

### Client

To create a client, we specify the message we will be using.

```rust
#[derive(Serialize, Deserialize)]
struct Msg {
    Hello,
    Goodbye,
}

/* on main */
let c = Client::connect(addr)?;
```

then we can send and receive messages

```rust
c.send(Msg::Hello)?;
match c.recv()? {
    Msg::Hello => { ... }
    Msg::Goodbye => { ... }
}
```

### Server

To create a server, we simply specify which messages we want to use, and which
state to represent connections.

```rust
struct State {
    value: i32,
}

/* on main */
let s = Server<State, Msg>::bind(addr)?;
```

you can then setup callback functions for different purposes, such as handling
new connections, or new messages, after that we can start the server.

```rust
s
    .on_connection(|conn| {})
    .on_message(|conn, msg| {
        // we can directly access the connection state
        conn.value += 1;
        // and also use connection methods
        conn.send(Msg::Goodbye).unwrap();
    })
    .run();
```

### Examples

You can try the example code by running `cargo run --example server` and then 
`cargo run --example client` in a different (or multiple) terminal(s), then
write commands to interact with the server.
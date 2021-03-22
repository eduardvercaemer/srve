//! This is the shared definitions for the client and server code.
#[derive(Serialize, Deserialize, Debug)]
pub enum Msg {
    /* client->server */
    Add(i32),
    Sub(i32),
    Print,

    /* server->client */
    Ok,
    Err,
    Value(i32),
}

#[derive(Debug)]
pub struct State {
    pub value: i32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            value: 0
        }
    }
}

pub const ADDR: &'static str = "127.0.0.1:6935";
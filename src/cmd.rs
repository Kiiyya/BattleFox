use std::sync::Mutex;

use ascii::AsciiString;
use battlefield_rcon::bf4::{Player, Visibility};
use futures::future::BoxFuture;
use multimap::MultiMap;

pub type CmdHandler<'a, R> = Box<dyn Fn() -> BoxFuture<'a, R>>;

struct Inner<'a> {
    cmds: MultiMap<String, CmdHandler<'a, ()>>,
}

impl<'a> Inner<'a> {
    pub fn new() -> Self {
        Self {
            cmds: MultiMap::new(),
        }
    }
}

pub struct Commands {
    inner: Mutex<Inner<'static>>,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner::new()),
        }
    }

    pub async fn handle_chat_msg(&self, _vis: &Visibility, _player: &Player, _msg: &AsciiString) {}
}

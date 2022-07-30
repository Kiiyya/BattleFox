//! Handling user commands and other chat goodies.

use std::sync::Arc;
use std::sync::Mutex;

use battlefield_rcon::bf4::Bf4Client;
use battlefield_rcon::bf4::Event::Chat;
use battlefield_rcon::rcon::RconResult;
use combine::Stream;
use futures::stream::StreamExt;
use combine::parser::token::Token;
use combine::stream::position;
use combine::{Parser, EasyParser, many1, sep_by, token};
use combine::parser::char::{letter, space};

pub trait Command {
    type Parameter;

    fn target(arg: Self::Parameter);
}

pub trait ParserCommand : Command {
    fn parser<Input: Stream>() -> Box<dyn Parser<Input, Output = Self::Parameter, PartialState = ()>>;
}

pub struct CmdLifetime {
    handle: usize,
    commands: Arc<Commands>,
}

impl Drop for CmdLifetime {
    fn drop(&mut self) {
        self.commands.cancel(self.handle);
    }
}

struct Inner {

}

impl Inner {
    fn rebuild() {
        debug!("Rebuilding commands parser")
    }
}

pub enum Matcher {
    /// For example `ban` will match `!ban PocketWolfy`, `/ban` etc.
    Exact(String),

    /// For example `pearl`
    ExactOrPrefix {
        prefix: String,
        minlen: usize,
    }
}

pub struct Commands {
    inner: Mutex<Inner>,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
            }),
        }
    }

    pub async fn run(&self, bf4: Arc<Bf4Client>) -> RconResult<()> {
        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                Ok(Chat { vis, player, msg }) => {

                },
                _ => (),
            }
        }

        Ok(())
    }

    pub fn add_stage() {

    }

    pub fn add_command() {

    }

    fn cancel(&self, handle: usize) {

    }
}


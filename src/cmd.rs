use std::{collections::HashMap, ops::Deref};

use futures::future::BoxFuture;

use crate::{ExtUp, Extension, Context, SomeScope, Scoped};

pub trait Chat {}

pub struct SimpleCommands {
    commands: HashMap<String, Box<dyn Fn()>>,
}

impl SimpleCommands {
    pub fn simple_command<F, S, St>(self: &St, firstword: impl Into<String>, f: F)
        where
            F: Fn(&[&str]) -> BoxFuture<'static, ()> + 'static,
            S: Chat + Context,
            St: Scoped<Self, S> + Deref<Target = Self>,
    {

    }
}

impl SimpleCommands {
    // pub fn has_command<'a>(
    //     &mut self,
    //     firstword: &'static str,
    //     cmd: impl Fn(&[&str]) -> BoxFuture<'static, Bf4Result<()>> + 'static,
    // ) -> CommandContribution {
    //     self.cmds.push((firstword, Box::new(cmd)));
    //     CommandContribution {}
    // }
}

impl Extension for SimpleCommands {
    fn define(scope: &mut impl ExtUp) -> Self
    where
        Self: Sized
    {
        todo!()
    }
}

pub struct CommandContribution {}

#[async_trait]
pub trait PlayerChatThing {
    async fn reply();
}

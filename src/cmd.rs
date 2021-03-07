use std::ops::Deref;

use futures::future::BoxFuture;

use crate::{ExtUp, Extension, Scope, SomeScope, State};

pub trait Chat {}

pub struct SimpleCommands {

}

impl SimpleCommands {
    pub fn has_command<F, S, St>(self: &St, firstword: &str, f: F)
        where
            F: Fn(&[&str]) -> BoxFuture<'static, ()> + 'static,
            S: Chat + Scope,
            St: State<Self, S> + Deref,
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
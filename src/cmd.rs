use std::{collections::HashMap, marker::PhantomData, ops::Deref};

use futures::future::BoxFuture;
use multimap::MultiMap;

use crate::{Node, Context};

pub trait Chat {}

pub trait SimpleCommandApi {
    fn has_command<'a>(
        &mut self,
        firstword: &'static str,
        cmd: impl Fn(&[&str]) -> BoxFuture<'static, Bf4Result<()>> + 'static,
    ) -> CommandContribution {
        self.cmds.push((firstword, Box::new(cmd)));
        CommandContribution {}
    }
}

pub struct SimpleCommands<C: Chat> {
    _x: PhantomData<C>,
    commands: MultiMap<String, Box<dyn Fn()>>,
}

// impl <C: Chat> SimpleCommands<C> {
//     pub fn simple_command<F, S, St>(self: &St, firstword: impl Into<String>, f: F)
//         where
//             F: Fn(&[&str]) -> BoxFuture<'static, ()> + 'static,
//             S: Chat + Context,
//             St: Scoped<Self, S> + Deref<Target = Self>,
//     {

//     }
// 

// impl SimpleCommands {
//     // pub fn has_command<'a>(
//     //     &mut self,
//     //     firstword: &'static str,
//     //     cmd: impl Fn(&[&str]) -> BoxFuture<'static, Bf4Result<()>> + 'static,
//     // ) -> CommandContribution {
//     //     self.cmds.push((firstword, Box::new(cmd)));
//     //     CommandContribution {}
//     // }
// }

impl <C: Context + Chat> Node for SimpleCommands<C> {
    type Ctx = C;

    fn define(ctx: &mut C) -> Self
    where
        Self: Sized
    {
        todo!()
    }
}

// #[async_trait]
// pub trait PlayerChatThing {
//     async fn reply();
// }

use std::{collections::HashMap, marker::PhantomData, ops::Deref};

use futures::future::BoxFuture;
use multimap::MultiMap;

pub struct SimpleCommands<C: Chat> {
    _x: PhantomData<C>,
    commands: MultiMap<String, Box<dyn Fn()>>,
}


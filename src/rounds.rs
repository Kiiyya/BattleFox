use battlefield_rcon::bf4::Event;

use crate::{Context, Node};


pub struct Rounds<'parent, Parent: Node> {
    parent: &'parent Parent,
    each_round: Vec<()>,
}

pub struct RoundsCtx {

}

impl Context for RoundsCtx {
    fn uses<'ctx, N: Node>(&'ctx mut self) -> &'ctx mut crate::Usage<N> {
        todo!()
    }
}

impl Node for Rounds {
    type Ctx = RoundsCtx;
    
    fn define(scope: &mut Self::Ctx) -> Self
    where
        Self: Sized
    {
        // scope.uses::<RawBf4Events>().with(|&evs| {
        //     evs.handler(|&ev: Event| {
        //         match ev {
        //             Event::RoundOver {winning_team} => { },
        //             _ => {},
        //         }
        //     });
        // });

        todo!()
    }
}
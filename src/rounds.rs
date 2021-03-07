use battlefield_rcon::bf4::Event;

use crate::Extension;


pub struct Rounds {

}

impl Extension for Rounds {
    // type Derp = ...;
    
    fn define(scope: &mut impl crate::ExtUp) -> Self
    where
        Self: Sized
    {
        scope.uses::<RawBf4Events>(|&evs| {
            evs.handler(|&ev: Event| {
                match ev {
                    Event::RoundOver {winning_team} => { },
                    _ => {},
                }
            });
        });
    }
}
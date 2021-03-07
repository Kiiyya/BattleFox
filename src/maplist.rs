use crate::{Extension, ExtUp};

pub struct Maplist {}

impl Extension for Maplist {
    fn up(scope: &mut impl ExtUp) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

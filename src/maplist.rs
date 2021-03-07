use crate::{Extension, ExtUp};

pub struct Maplist {}

impl Extension for Maplist {
    fn define(scope: &mut impl ExtUp) -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

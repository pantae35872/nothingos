#![no_std]

use core::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Items<Xs>(pub Xs);

impl<Xs: Clone + Iterator> fmt::Display for Items<Xs> where Xs::Item: fmt::Display {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for x in self.0.clone() { write!(f, "{}", x)?; }
        Ok(())
    }
}

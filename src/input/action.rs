use crate::engine::Engine;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionID(pub Cow<'static, str>);

impl From<&'static str> for ActionID {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<String> for ActionID {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

pub type ActionCallback<Ctx = Engine> = Box<dyn FnMut(&mut Ctx)>;

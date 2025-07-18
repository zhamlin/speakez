use std::num::{NonZero, NonZeroU32};

/// A session represents a unique ID for a given user.
// Session IDs can be resued?
// 0 is not a valid session in mumble
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Session(NonZeroU32);

impl Session {
    pub fn new(value: u32) -> Option<Self> {
        Some(Session(NonZeroU32::new(value)?))
    }
}

impl From<Session> for u32 {
    fn from(val: Session) -> Self {
        val.0.into()
    }
}

#[derive(Debug)]
pub struct Sessions {
    data: Vec<Session>,
}

impl Sessions {
    pub fn new(n: usize) -> Self {
        let mut data = Vec::with_capacity(n);
        for i in (1..n + 1).rev() {
            let s = Session::new(i.try_into().unwrap()).expect("session should not have 0 value");
            data.push(s);
        }

        Sessions { data }
    }

    pub fn get_session(&mut self) -> Option<Session> {
        self.data.pop()
    }

    pub fn return_session(&mut self, s: Session) {
        self.data.push(s)
    }
}

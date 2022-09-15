use std::{
    fmt::{self, Display},
    ops::{Add, Deref, DerefMut},
    string::String as RustString,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct String {
    inner: RustString,
}

impl String {
    pub fn new(chars: &str) -> Self {
        Self {
            inner: RustString::from(chars),
        }
    }
}

impl Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;

        Ok(())
    }
}

impl Add for &String {
    type Output = String;

    fn add(self, rhs: Self) -> Self::Output {
        String {
            inner: format!("{}{}", self.inner, rhs.inner),
        }
    }
}

impl Deref for String {
    type Target = RustString;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    NotSerializingStruct,
    Serde(String),
    UnsignedIntNotInSpec,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSerializingStruct => write!(
                f,
                "individual values cannot be serialised, try serialising a struct instead"
            ),
            Self::Serde(context) => write!(f, "error from value serialiser: {}", context),
            Self::UnsignedIntNotInSpec => {
                write!(f, "unsigned ints are not supported in the bson spec")
            }
        }
    }
}

impl std::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serde(msg.to_string())
    }
}

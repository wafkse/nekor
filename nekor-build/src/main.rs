//! The build orchestration crate for the Nekor Unikernel.

use core::{
    error::Error,
    fmt::{self, Debug, Display},
};

use clap::Parser;
use nekor_build::invoke::{Invoke, InvokeError};

/// A new-type that forwards the [`Debug`] implementation as the [`Display`]
/// one.
#[repr(transparent)]
struct DisplayAsDebug<D>(D)
where
    D: Display;

impl<D> Error for DisplayAsDebug<D>
where
    D: Display + Error,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        let &Self(ref source_error) = self;

        source_error.source()
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl<D> Debug for DisplayAsDebug<D>
where
    D: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value) = self;

        <D as Display>::fmt(target_value, f)
    }
}

impl<D> Display for DisplayAsDebug<D>
where
    D: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value) = self;

        <D as Display>::fmt(target_value, f)
    }
}

/// A newtype that takes an error and forwards it as an error-chain for its
/// [`Debug`] implementation.
#[repr(transparent)]
struct ErrorChain<E>(E)
where
    E: Error;

impl<E> Display for ErrorChain<E>
where
    E: Error + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self(ref target_value) = self;

        writeln!(f, "{target_value}")?;

        let mut target_error = target_value.source();

        while let Some(source_error) = target_error {
            writeln!(f, "Caused by:\n\t{source_error}")?;

            target_error = source_error.source();
        }

        Ok(())
    }
}

fn main() -> Result<(), DisplayAsDebug<ErrorChain<InvokeError>>> {
    Invoke::parse().run().map_err(ErrorChain).map_err(DisplayAsDebug)
}

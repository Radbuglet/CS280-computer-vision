//! Error reporting built of the Rust standard library [Error] trait.
//! Stolen from [Crucible](crucible-error).
//!
//! [crucible-error]: https://github.com/Radbuglet/crucible/blob/69d6a91ada5b5323a4dab071b279d65caeec91dd/src/client/src/util/error.rs

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

pub type AnyResult<T> = anyhow::Result<T>;
pub type AnyError = anyhow::Error;

pub trait ErrorFormatExt {
    fn format_error(&self) -> FormattedError<Self>;
}

impl<T: ?Sized + Error> ErrorFormatExt for T {
    fn format_error(&self) -> FormattedError<Self> {
        FormattedError { target: self }
    }
}

pub struct FormattedError<'a, T: ?Sized> {
    target: &'a T,
}

impl<T: ?Sized + Error> Display for FormattedError<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let target = self.target;

        // Write context
        writeln!(f, "Error: {}", target)?;

        // Write cause chain
        // (we iterate manually instead of using `anyhow::Chain` because it consumes a `&dyn Error`.
        {
            let mut cause_iter = target.source();
            if cause_iter.is_some() {
                writeln!(f, "\nCaused by:")?;
            }

            while let Some(cause) = cause_iter {
                writeln!(f, "\t {}", cause)?;
                cause_iter = cause.source();
            }
        }

        Ok(())
    }
}

/// A version of [anyhow::Context] for [Result] only. Supports producing context from an error value.
pub trait ResultContext<T, E> {
    fn with_context_proc<C, F>(self, f: F) -> Result<T, AnyError>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce(&E) -> C;
}

impl<T, E> ResultContext<T, E> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    fn with_context_proc<C, F>(self, f: F) -> Result<T, AnyError>
    where
        C: Display + Send + Sync + 'static,
        F: FnOnce(&E) -> C,
    {
        self.map_err(|err| {
            let ctx = f(&err);
            AnyError::new(err).context(ctx)
        })
    }
}

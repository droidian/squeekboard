/* Copyright (C) 2020-2021 Purism SPC
 * SPDX-License-Identifier: GPL-3.0+
 */

/*! Combined module for dealing with layout files */

mod loading;
pub mod parsing;

use std::io;
use std::fmt;

use ::keyboard::FormattingError;

/// Errors encountered loading the layout into yaml
#[derive(Debug)]
pub enum Error {
    Yaml(serde_yaml::Error),
    Io(io::Error),
    /// The file was missing.
    /// It's distinct from Io in order to make it matchable
    /// without calling io::Error::kind()
    Missing(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Yaml(e) => write!(f, "YAML: {}", e),
            Error::Io(e) => write!(f, "IO: {}", e),
            Error::Missing(e) => write!(f, "Missing: {}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        let kind = e.kind();
        match kind {
            io::ErrorKind::NotFound => Error::Missing(e),
            _ => Error::Io(e),
        }
    }
}


#[derive(Debug)]
pub enum LoadError {
    BadData(Error),
    MissingResource,
    BadResource(serde_yaml::Error),
    BadKeyMap(FormattingError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::LoadError::*;
        match self {
            BadData(e) => write!(f, "Bad data: {}", e),
            MissingResource => write!(f, "Missing resource"),
            BadResource(e) => write!(f, "Bad resource: {}", e),
            BadKeyMap(e) => write!(f, "Bad key map: {}", e),
        }
    }
}

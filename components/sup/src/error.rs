// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Error handling for the Supervisor.
//!
//! Errors in the Supervisor are of the type `SupError`, which contains an `Error` along with
//! information about where the error was created in the code base, in the same way that the
//! `output` module does. To simplify the creation of these annotated errors, we provide the
//! `sup_error!` macro, which takes only an `Error` as its argument.
//!
//! To match on `Error`, do something like this:
//!
//! ```ignore
//! let error = sup_error!(Error::CommandNotImplemented);
//! let result = match error {
//!     SupError{err: Error::CommandNotImplemented, ..} => true,
//!     _ => false
//! };
//! assert_eq!(result, true);
//! ```
//!
//! When printing errors, we automatically create a `StructuredOutput` with the `verbose` flag set,
//! ensuring that you can see the file, line number, and column it was created from.
//!
//! Also included in this module is `Result<T>`, a type alias for `Result<T, SupError>`. Use
//! it instead of the longer `Result` form.

use std::io;
use std::env;
use std::error;
use std::ffi;
use std::fmt;
use std::net;
use std::num;
use std::path::PathBuf;
use std::result;
use std::str;
use std::string;
use std::sync::mpsc;

use ansi_term::Colour::Red;
use butterfly;
use common;
use depot_client;
use glob;
use handlebars;
use hcore::{self, package};
use hcore::package::Identifiable;
use notify;
use toml;

use output::StructuredOutput;
use PROGRAM_NAME;

static LOGKEY: &'static str = "ER";

/// Our result type alias, for easy coding.
pub type Result<T> = result::Result<T, SupError>;

#[derive(Debug)]
/// All errors in the Supervisor are kept in this struct. We store `Error`, an enum with a variant
/// for every type of error we produce. It also stores the location the error was created.
pub struct SupError {
    pub err: Error,
    logkey: &'static str,
    file: &'static str,
    line: u32,
    column: u32,
}

impl SupError {
    /// Create a new `SupError`. Usually accessed through the `sup_error!` macro, rather than
    /// called directly.
    pub fn new(err: Error,
               logkey: &'static str,
               file: &'static str,
               line: u32,
               column: u32)
               -> SupError {
        SupError {
            err: err,
            logkey: logkey,
            file: file,
            line: line,
            column: column,
        }
    }
}

/// All the kinds of errors we produce.
#[derive(Debug)]
pub enum Error {
    BadDataFile(PathBuf, io::Error),
    BadDataPath(PathBuf, io::Error),
    BadSpecsPath(PathBuf, io::Error),
    ButterflyError(butterfly::error::Error),
    CommandNotImplemented,
    DbInvalidPath,
    DepotClient(depot_client::Error),
    EnvJoinPathsError(env::JoinPathsError),
    ExecCommandNotFound(String),
    FileNotFound(String),
    HabitatCommon(common::Error),
    HabitatCore(hcore::Error),
    TemplateFileError(handlebars::TemplateFileError),
    TemplateRenderError(handlebars::RenderError),
    InvalidBinding(String),
    InvalidKeyParameter(String),
    InvalidPidFile,
    InvalidPort(num::ParseIntError),
    InvalidServiceGroupString(String),
    InvalidTopology(String),
    InvalidUpdateStrategy(String),
    Io(io::Error),
    IPFailed,
    KeyNotFound(String),
    MetaFileIO(io::Error),
    MissingRequiredBind(Vec<String>),
    MissingRequiredIdent,
    NameLookup(io::Error),
    NetParseError(net::AddrParseError),
    NoRunFile,
    NoProcessLock,
    NulError(ffi::NulError),
    PackageArchiveMalformed(String),
    PackageNotFound(package::PackageIdent),
    Permissions(String),
    RemotePackageNotFound(package::PackageIdent),
    RootRequired,
    ServiceSpecFileRead(String, String),
    ServiceSpecFileWrite(String, String),
    ServiceSpecParse(String),
    ServiceSpecRender(String),
    SignalFailed,
    SignalNotifierStarted,
    SpecWatcherDirNotFound(String),
    SpecWatcherGlob(glob::PatternError),
    SpecWatcherNotify(notify::Error),
    StrFromUtf8Error(str::Utf8Error),
    StringFromUtf8Error(string::FromUtf8Error),
    TomlEncode(toml::ser::Error),
    TomlMergeError(String),
    TomlParser(toml::de::Error),
    TryRecvError(mpsc::TryRecvError),
    UnpackFailed,
}

impl fmt::Display for SupError {
    // We create a string for each type of error, then create a `StructuredOutput` for it, flip
    // verbose on, and print it.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let content = match self.err {
            Error::BadDataFile(ref path, ref err) => {
                format!("Unable to read or write to data file, {}, {}",
                        path.display(),
                        err)
            }
            Error::BadDataPath(ref path, ref err) => {
                format!("Unable to read or write to data directory, {}, {}",
                        path.display(),
                        err)
            }
            Error::BadSpecsPath(ref path, ref err) => {
                format!("Unable to create the specs directory '{}' ({})",
                        path.display(),
                        err)
            }
            Error::ButterflyError(ref err) => format!("Butterfly error: {}", err),
            Error::ExecCommandNotFound(ref c) => {
                format!("`{}' was not found on the filesystem or in PATH", c)
            }
            Error::Permissions(ref err) => format!("{}", err),
            Error::HabitatCommon(ref err) => format!("{}", err),
            Error::HabitatCore(ref err) => format!("{}", err),
            Error::TemplateFileError(ref err) => format!("{:?}", err),
            Error::TemplateRenderError(ref err) => format!("{}", err),
            Error::CommandNotImplemented => format!("Command is not yet implemented!"),
            Error::DbInvalidPath => format!("Invalid filepath to internal datastore"),
            Error::DepotClient(ref err) => format!("{}", err),
            Error::EnvJoinPathsError(ref err) => format!("{}", err),
            Error::FileNotFound(ref e) => format!("File not found at: {}", e),
            Error::InvalidBinding(ref binding) => {
                format!("Invalid binding \"{}\", must be of the form <NAME>:<SERVICE_GROUP> where \
                         <NAME> is a service name and <SERVICE_GROUP> is a valid service group",
                        binding)
            }
            Error::InvalidKeyParameter(ref e) => {
                format!("Invalid parameter for key generation: {:?}", e)
            }
            Error::InvalidPort(ref e) => {
                format!("Invalid port number in package expose metadata: {}", e)
            }
            Error::InvalidPidFile => format!("Invalid child process PID file"),
            Error::InvalidServiceGroupString(ref e) => {
                format!("Invalid service group string: {}", e)
            }
            Error::InvalidTopology(ref t) => format!("Invalid topology: {}", t),
            Error::InvalidUpdateStrategy(ref s) => format!("Invalid update strategy: {}", s),
            Error::Io(ref err) => format!("{}", err),
            Error::IPFailed => format!("Failed to discover this hosts outbound IP address"),
            Error::KeyNotFound(ref e) => format!("Key not found in key cache: {}", e),
            Error::MissingRequiredBind(ref e) => {
                format!("Missing required bind(s), {}", e.join(", "))
            }
            Error::MissingRequiredIdent => format!("Missing required ident field: (example: ident = \"core/redis\")"),
            Error::MetaFileIO(ref e) => format!("IO error while accessing MetaFile: {:?}", e),
            Error::NameLookup(ref e) => format!("Error resolving a name or IP address: {}", e),
            Error::NetParseError(ref e) => format!("Can't parse ip:port: {}", e),
            Error::NoRunFile => {
                format!("No run file is present for this package; specify a run hook or \
                         $pkg_svc_run in your plan")
            }
            Error::NoProcessLock => {
                format!("Unable to obtain a mutually exclusive process lock. This typically \
                    happens if a previously running Supervisor was unable to cleanup \
                    `/hab/sup/default/LOCK` before terminating or if you are attempting to run \
                    multiple Supervisors. Running multiple Supervisors is not recommended, but \
                    can be done by setting a value for `--override-name`")
            }
            Error::NulError(ref e) => format!("{}", e),
            Error::PackageArchiveMalformed(ref e) => {
                format!("Package archive was unreadable or contained unexpected contents: {:?}",
                        e)
            }
            Error::PackageNotFound(ref pkg) => {
                if pkg.fully_qualified() {
                    format!("Cannot find package: {}", pkg)
                } else {
                    format!("Cannot find a release of package: {}", pkg)
                }
            }
            Error::RemotePackageNotFound(ref pkg) => {
                if pkg.fully_qualified() {
                    format!("Cannot find package in any sources: {}", pkg)
                } else {
                    format!("Cannot find a release of package in any sources: {}", pkg)
                }
            }
            Error::RootRequired => {
                "Root or administrator permissions required to complete operation".to_string()
            }
            Error::ServiceSpecFileRead(ref path, ref details) => {
                format!("Service spec file '{}' could not be read successfully: {}",
                        path,
                        details)
            }
            Error::ServiceSpecFileWrite(ref path, ref details) => {
                format!("Service spec file '{}' could not be written successfully: {}",
                        path,
                        details)
            }
            Error::ServiceSpecParse(ref details) => {
                format!("Service spec could not be parsed successfully: {}", details)
            }
            Error::ServiceSpecRender(ref details) => {
                format!("Service spec TOML could not be rendered successfully: {}",
                        details)
            }
            Error::SignalFailed => format!("Failed to send a signal to the child process"),
            Error::SignalNotifierStarted => format!("Only one instance of a Signal Notifier may be running"),
            Error::SpecWatcherDirNotFound(ref path) => {
                format!("Spec directory '{}' not created or is not a directory",
                        path)
            }
            Error::SpecWatcherGlob(ref e) => format!("{}", e),
            Error::SpecWatcherNotify(ref e) => format!("{}", e),
            Error::StrFromUtf8Error(ref e) => format!("{}", e),
            Error::StringFromUtf8Error(ref e) => format!("{}", e),
            Error::TomlEncode(ref e) => format!("Failed to encode TOML: {}", e),
            Error::TomlMergeError(ref e) => format!("Failed to merge TOML: {}", e),
            Error::TomlParser(ref err) => format!("Failed to parse TOML: {}", err),
            Error::TryRecvError(ref err) => format!("{}", err),
            Error::UnpackFailed => format!("Failed to unpack a package"),
        };
        let cstring = Red.bold().paint(content).to_string();
        let progname = PROGRAM_NAME.as_str();
        let mut so = StructuredOutput::new(progname,
                                           self.logkey,
                                           self.line,
                                           self.file,
                                           self.column,
                                           &cstring);
        so.verbose = Some(true);
        write!(f, "{}", so)
    }
}

impl error::Error for SupError {
    fn description(&self) -> &str {
        match self.err {
            Error::BadDataFile(_, _) => "Unable to read or write to a data file",
            Error::BadDataPath(_, _) => "Unable to read or write to data directory",
            Error::BadSpecsPath(_, _) => "Unable to create the specs directory",
            Error::ButterflyError(ref err) => err.description(),
            Error::ExecCommandNotFound(_) => "Exec command was not found on filesystem or in PATH",
            Error::TemplateFileError(ref err) => err.description(),
            Error::TemplateRenderError(ref err) => err.description(),
            Error::HabitatCommon(ref err) => err.description(),
            Error::HabitatCore(ref err) => err.description(),
            Error::CommandNotImplemented => "Command is not yet implemented!",
            Error::DbInvalidPath => "A bad filepath was provided for an internal datastore",
            Error::DepotClient(ref err) => err.description(),
            Error::EnvJoinPathsError(ref err) => err.description(),
            Error::FileNotFound(_) => "File not found",
            Error::InvalidBinding(_) => "Invalid binding parameter",
            Error::InvalidKeyParameter(_) => "Key parameter error",
            Error::InvalidPort(_) => "Invalid port number in package expose metadata",
            Error::InvalidPidFile => "Invalid child process PID file",
            Error::InvalidServiceGroupString(_) => "Service group strings must be in service.group format (example: redis.default)",
            Error::InvalidTopology(_) => "Invalid topology",
            Error::InvalidUpdateStrategy(_) => "Invalid update strategy",
            Error::Io(ref err) => err.description(),
            Error::IPFailed => "Failed to discover the outbound IP address",
            Error::KeyNotFound(_) => "Key not found in key cache",
            Error::MissingRequiredBind(_) => "A service to start without specifying a service group for all required binds",
            Error::MissingRequiredIdent => "Missing required ident field: (example: ident = \"core/redis\")",
            Error::MetaFileIO(_) => "MetaFile could not be read or written to",
            Error::NetParseError(_) => "Can't parse IP:port",
            Error::NameLookup(_) => "Error resolving a name or IP address",
            Error::NoRunFile => {
                "No run file is present for this package; specify a run hook or $pkg_svc_run \
                 in your plan"
            }
            Error::NoProcessLock => "Unable to obtain a mutual exclusive process lock",
            Error::NulError(_) => "An attempt was made to build a CString with a null byte inside it",
            Error::PackageArchiveMalformed(_) => "Package archive was unreadable or had unexpected contents",
            Error::PackageNotFound(_) => "Cannot find a package",
            Error::Permissions(_) => "File system permissions error",
            Error::RemotePackageNotFound(_) => "Cannot find a package in any sources",
            Error::RootRequired => "Root or administrator permissions required to complete operation",
            Error::ServiceSpecFileRead(_, _) => "Service spec file could not be read successfully",
            Error::ServiceSpecFileWrite(_, _) => "Service spec file could not be written successfully",
            Error::ServiceSpecParse(_) => "Service spec could not be parsed successfully",
            Error::ServiceSpecRender(_) => "Service spec TOML could not be rendered successfully",
            Error::SignalFailed => "Failed to send a signal to the child process",
            Error::SignalNotifierStarted => "Only one instance of a Signal Notifier may be running",
            Error::SpecWatcherDirNotFound(_) => "Spec directory not created or is not a directory",
            Error::SpecWatcherGlob(_) => "Spec watcher file globbing error",
            Error::SpecWatcherNotify(_) => "Spec watcher error",
            Error::StrFromUtf8Error(_) => "Failed to convert a str from a &[u8] as UTF-8",
            Error::StringFromUtf8Error(_) => "Failed to convert a string from a Vec<u8> as UTF-8",
            Error::TomlEncode(_) => "Failed to encode toml!",
            Error::TomlMergeError(_) => "Failed to merge TOML!",
            Error::TomlParser(_) => "Failed to parse TOML!",
            Error::TryRecvError(_) => "A channel failed to receive a response",
            Error::UnpackFailed => "Failed to unpack a package",
        }
    }
}

impl From<net::AddrParseError> for SupError {
    fn from(err: net::AddrParseError) -> SupError {
        sup_error!(Error::NetParseError(err))
    }
}

impl From<butterfly::error::Error> for SupError {
    fn from(err: butterfly::error::Error) -> SupError {
        sup_error!(Error::ButterflyError(err))
    }
}

impl From<common::Error> for SupError {
    fn from(err: common::Error) -> SupError {
        sup_error!(Error::HabitatCommon(err))
    }
}

impl From<glob::PatternError> for SupError {
    fn from(err: glob::PatternError) -> SupError {
        sup_error!(Error::SpecWatcherGlob(err))
    }
}

impl From<handlebars::RenderError> for SupError {
    fn from(err: handlebars::RenderError) -> SupError {
        sup_error!(Error::TemplateRenderError(err))
    }
}

impl From<handlebars::TemplateFileError> for SupError {
    fn from(err: handlebars::TemplateFileError) -> SupError {
        sup_error!(Error::TemplateFileError(err))
    }
}

impl From<hcore::Error> for SupError {
    fn from(err: hcore::Error) -> SupError {
        sup_error!(Error::HabitatCore(err))
    }
}

impl From<depot_client::Error> for SupError {
    fn from(err: depot_client::Error) -> SupError {
        sup_error!(Error::DepotClient(err))
    }
}

impl From<ffi::NulError> for SupError {
    fn from(err: ffi::NulError) -> SupError {
        sup_error!(Error::NulError(err))
    }
}

impl From<io::Error> for SupError {
    fn from(err: io::Error) -> SupError {
        sup_error!(Error::Io(err))
    }
}

impl From<env::JoinPathsError> for SupError {
    fn from(err: env::JoinPathsError) -> SupError {
        sup_error!(Error::EnvJoinPathsError(err))
    }
}

impl From<string::FromUtf8Error> for SupError {
    fn from(err: string::FromUtf8Error) -> SupError {
        sup_error!(Error::StringFromUtf8Error(err))
    }
}

impl From<str::Utf8Error> for SupError {
    fn from(err: str::Utf8Error) -> SupError {
        sup_error!(Error::StrFromUtf8Error(err))
    }
}

impl From<mpsc::TryRecvError> for SupError {
    fn from(err: mpsc::TryRecvError) -> SupError {
        sup_error!(Error::TryRecvError(err))
    }
}

impl From<notify::Error> for SupError {
    fn from(err: notify::Error) -> SupError {
        sup_error!(Error::SpecWatcherNotify(err))
    }
}

impl From<toml::de::Error> for SupError {
    fn from(err: toml::de::Error) -> Self {
        sup_error!(Error::TomlParser(err))
    }
}

impl From<toml::ser::Error> for SupError {
    fn from(err: toml::ser::Error) -> Self {
        sup_error!(Error::TomlEncode(err))
    }
}

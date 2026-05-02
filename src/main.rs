#![forbid(unsafe_code)]
//! `whoneeds` command-line entry point.
//!
//! The command preserves the historical `whoneeds <package>` user interface while
//! delegating reverse-dependency discovery to Arch's native package tooling.

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::process::{Command, ExitCode};

const EXIT_SUCCESS: u8 = 0;
const EXIT_NO_DEPENDENTS: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_BACKEND: u8 = 3;

#[derive(Debug, Eq, PartialEq)]
struct PackageName(String);

impl PackageName {
    fn new(value: String) -> Self {
        Self(value)
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Eq, PartialEq)]
enum CliError {
    UnexpectedArgumentCount,
    Backend(BackendError),
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedArgumentCount => {
                write!(formatter, "error: unexpected number of arguments")
            }
            Self::Backend(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for CliError {}

#[derive(Debug, Eq, PartialEq)]
struct BackendError {
    package: String,
}

impl BackendError {
    fn new(package: &PackageName) -> Self {
        Self {
            package: package.as_str().to_owned(),
        }
    }
}

impl Display for BackendError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "error: could not get information on {}",
            self.package
        )
    }
}

impl std::error::Error for BackendError {}

#[derive(Debug, Eq, PartialEq)]
enum Dependents {
    Some(Vec<String>),
    None,
}

impl Dependents {
    fn from_sets(
        package: &PackageName,
        reverse_dependencies: &BTreeSet<String>,
        explicit_packages: &BTreeSet<String>,
    ) -> Self {
        let packages = reverse_dependencies
            .intersection(explicit_packages)
            .filter(|candidate| candidate.as_str() != package.as_str())
            .cloned()
            .collect::<Vec<_>>();

        if packages.is_empty() {
            Self::None
        } else {
            Self::Some(packages)
        }
    }
}

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(status) => status,
        Err(CliError::UnexpectedArgumentCount) => {
            eprintln!("error: unexpected number of arguments");
            println!("Usage: whoneeds <package-name>");
            ExitCode::from(EXIT_USAGE)
        }
        Err(CliError::Backend(error)) => {
            eprintln!("{error}");
            ExitCode::from(EXIT_BACKEND)
        }
    }
}

fn run(mut args: impl Iterator<Item = String>) -> Result<ExitCode, CliError> {
    let Some(package) = args.next() else {
        return Err(CliError::UnexpectedArgumentCount);
    };

    if args.next().is_some() {
        return Err(CliError::UnexpectedArgumentCount);
    }

    let package = PackageName::new(package);
    let dependents = find_dependents(&package).map_err(CliError::Backend)?;
    print_dependents(&package, &dependents);

    match dependents {
        Dependents::Some(_) => Ok(ExitCode::from(EXIT_SUCCESS)),
        Dependents::None => Ok(ExitCode::from(EXIT_NO_DEPENDENTS)),
    }
}

fn find_dependents(package: &PackageName) -> Result<Dependents, BackendError> {
    let reverse_dependencies = command_lines("pactree", ["-lru", package.as_str()])
        .map_err(|()| BackendError::new(package))?;
    let explicit_packages =
        command_lines("pacman", ["-Qqe"]).map_err(|()| BackendError::new(package))?;

    Ok(Dependents::from_sets(
        package,
        &reverse_dependencies,
        &explicit_packages,
    ))
}

fn command_lines<const N: usize>(program: &str, args: [&str; N]) -> Result<BTreeSet<String>, ()> {
    let output = Command::new(program)
        .args(args.iter().map(OsStr::new))
        .output()
        .map_err(|_| ())?;

    if !output.status.success() {
        return Err(());
    }

    let stdout = String::from_utf8(output.stdout).map_err(|_| ())?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn print_dependents(package: &PackageName, dependents: &Dependents) {
    println!("Packages that depend on [{}]", package.as_str());

    match dependents {
        Dependents::Some(packages) => {
            for package in packages {
                println!("  {package}");
            }
        }
        Dependents::None => println!("  None"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Dependents, PackageName};
    use std::collections::BTreeSet;

    #[test]
    fn filters_to_explicit_reverse_dependencies_and_excludes_query_package() {
        let package = PackageName::new("zlib".to_owned());
        let reverse_dependencies =
            BTreeSet::from(["curl".to_owned(), "pacman".to_owned(), "zlib".to_owned()]);
        let explicit_packages = BTreeSet::from(["pacman".to_owned(), "zlib".to_owned()]);

        assert_eq!(
            Dependents::from_sets(&package, &reverse_dependencies, &explicit_packages),
            Dependents::Some(vec!["pacman".to_owned()])
        );
    }

    #[test]
    fn reports_no_dependents_when_only_query_package_is_explicit() {
        let package = PackageName::new("zlib".to_owned());
        let reverse_dependencies = BTreeSet::from(["zlib".to_owned()]);
        let explicit_packages = BTreeSet::from(["zlib".to_owned()]);

        assert_eq!(
            Dependents::from_sets(&package, &reverse_dependencies, &explicit_packages),
            Dependents::None
        );
    }
}

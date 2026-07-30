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
    Some {
        explicit: Vec<String>,
        automatic: Vec<String>,
    },
    None,
}

impl Dependents {
    fn from_sets(
        package: &PackageName,
        mut reverse_dependencies: BTreeSet<String>,
        explicit_packages: &BTreeSet<String>,
    ) -> Self {
        reverse_dependencies.remove(package.as_str());

        let (explicit, automatic): (Vec<String>, Vec<String>) = reverse_dependencies
            .into_iter()
            .partition(|candidate| explicit_packages.contains(candidate));

        if explicit.is_empty() && automatic.is_empty() {
            Self::None
        } else {
            Self::Some {
                explicit,
                automatic,
            }
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
        Dependents::Some { .. } => Ok(ExitCode::from(EXIT_SUCCESS)),
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
        reverse_dependencies,
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
    for line in dependent_output_lines(package, dependents) {
        println!("{line}");
    }
}

fn dependent_output_lines(package: &PackageName, dependents: &Dependents) -> Vec<String> {
    match dependents {
        Dependents::Some {
            explicit,
            automatic,
        } => {
            let mut lines = Vec::with_capacity(explicit.len() + automatic.len() + 2);

            if !explicit.is_empty() {
                lines.push(format!(
                    "Explicitly installed packages that depend on [{}]",
                    package.as_str()
                ));
                lines.extend(explicit.iter().map(|package| format!("  {package}")));
            }

            if !automatic.is_empty() {
                lines.push(format!(
                    "Other installed packages that depend on [{}]",
                    package.as_str()
                ));
                lines.extend(automatic.iter().map(|package| format!("  {package}")));
            }

            lines
        }
        Dependents::None => vec![
            format!("Packages that depend on [{}]", package.as_str()),
            "  None".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{dependent_output_lines, Dependents, PackageName};
    use std::collections::BTreeSet;

    #[test]
    fn separates_explicit_and_automatic_reverse_dependencies() {
        let package = PackageName::new("target".to_owned());
        let reverse_dependencies = BTreeSet::from([
            "automatic-a".to_owned(),
            "explicit-a".to_owned(),
            "explicit-b".to_owned(),
            "target".to_owned(),
        ]);
        let explicit_packages = BTreeSet::from(["explicit-a".to_owned(), "explicit-b".to_owned()]);

        assert_eq!(
            Dependents::from_sets(&package, reverse_dependencies, &explicit_packages),
            Dependents::Some {
                explicit: vec!["explicit-a".to_owned(), "explicit-b".to_owned()],
                automatic: vec!["automatic-a".to_owned()],
            }
        );
    }

    #[test]
    fn renders_both_non_empty_categories() {
        let package = PackageName::new("target".to_owned());
        let dependents = Dependents::Some {
            explicit: vec!["explicit-a".to_owned()],
            automatic: vec!["automatic-a".to_owned()],
        };

        assert_eq!(
            dependent_output_lines(&package, &dependents),
            vec![
                "Explicitly installed packages that depend on [target]".to_owned(),
                "  explicit-a".to_owned(),
                "Other installed packages that depend on [target]".to_owned(),
                "  automatic-a".to_owned(),
            ]
        );
    }

    #[test]
    fn omits_empty_categories() {
        let package = PackageName::new("target".to_owned());
        let dependents = Dependents::Some {
            explicit: Vec::new(),
            automatic: vec!["automatic-a".to_owned()],
        };

        assert_eq!(
            dependent_output_lines(&package, &dependents),
            vec![
                "Other installed packages that depend on [target]".to_owned(),
                "  automatic-a".to_owned(),
            ]
        );
    }

    #[test]
    fn renders_only_explicit_category_when_no_automatic_dependents_exist() {
        let package = PackageName::new("target".to_owned());
        let dependents = Dependents::Some {
            explicit: vec!["explicit-a".to_owned()],
            automatic: Vec::new(),
        };

        assert_eq!(
            dependent_output_lines(&package, &dependents),
            vec![
                "Explicitly installed packages that depend on [target]".to_owned(),
                "  explicit-a".to_owned(),
            ]
        );
    }

    #[test]
    fn reports_none_when_no_reverse_dependencies_remain() {
        let package = PackageName::new("target".to_owned());
        let reverse_dependencies = BTreeSet::from(["target".to_owned()]);
        let explicit_packages = BTreeSet::new();
        let dependents = Dependents::from_sets(&package, reverse_dependencies, &explicit_packages);

        assert_eq!(dependents, Dependents::None);
        assert_eq!(
            dependent_output_lines(&package, &dependents),
            vec![
                "Packages that depend on [target]".to_owned(),
                "  None".to_owned(),
            ]
        );
    }
}

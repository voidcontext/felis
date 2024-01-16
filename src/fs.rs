use std::path::{Path, PathBuf};

use crate::{command, Environment};

pub struct AbsolutePath {
    buf: PathBuf,
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.buf.as_path()
    }
}

impl TryFrom<&str> for AbsolutePath {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let buf = PathBuf::from(value);
        if buf.is_absolute() {
            Ok(Self { buf })
        } else {
            Err("Given path is not absolute!".to_owned())
        }
    }
}

impl TryFrom<PathBuf> for AbsolutePath {
    type Error = String;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self { buf: value })
        } else {
            Err("Given path is not absolute!".to_owned())
        }
    }
}

impl AbsolutePath {
    /// # Errors
    ///
    /// Returns error when in a terminal environment we cannot find the active focused window
    pub fn resolve<P: AsRef<Path>>(path: &P, env: &Environment) -> crate::Result<Self> {
        if path.as_ref().is_absolute() {
            Ok(Self {
                buf: PathBuf::from(path.as_ref()),
            })
        } else {
            // In theory these to branches should give the same result when
            // executed from a shell context, since the active focused window should be the one
            // where we're runnning the felis command in the shell.
            //
            // In practice, it's easier the reason about the correctness of the paths when make a
            // distinction between the 2 contexts, not to mention that we can avoid a call to the
            // terminal when we're in a shell context
            match env {
                Environment::Shell(cwd) => {
                    let mut buf = PathBuf::new();
                    buf.push(cwd);
                    buf.push(path.as_ref());
                    Ok(Self { buf })
                }
                Environment::Kitty(windows) => command::focused_active_window(windows).map_or(
                    Err(crate::FelisError::UnexpectedError {
                        message: "Couldn't find active focused window".to_owned(),
                    }),
                    |window| {
                        let mut buf = PathBuf::new();
                        buf.push(command::window_cwd(window));
                        buf.push(path.as_ref());
                        Ok(Self { buf })
                    },
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use kitty_remote_bindings::model::OsWindows;
    use pretty_assertions::assert_eq;

    use crate::{kitty_terminal::test_fixture, Environment};

    use super::AbsolutePath;

    #[test]
    fn test_absolute_path_resolve_should_return_path_if_absolute_in_shell_env() {
        let result = AbsolutePath::resolve(
            &Path::new("/path/to/file.txt"),
            &Environment::Shell(PathBuf::new()),
        )
        .unwrap();

        assert_eq!(result.buf, PathBuf::from("/path/to/file.txt"));
    }

    #[test]
    fn test_absolute_path_resolve_should_return_path_if_absolute_in_kitty_env() {
        let result = AbsolutePath::resolve(
            &Path::new("/path/to/file.txt"),
            &Environment::Kitty(OsWindows(Vec::new())),
        )
        .unwrap();

        assert_eq!(result.buf, PathBuf::from("/path/to/file.txt"));
    }

    #[test]
    fn test_absolute_path_resolve_should_return_path_based_on_cwd_in_shell_env() {
        let result = AbsolutePath::resolve(
            &Path::new("file.txt"),
            &Environment::Shell(PathBuf::from("/path/to/work_dir/")),
        )
        .unwrap();

        assert_eq!(
            result.buf,
            PathBuf::from(Path::new("/path/to/work_dir/file.txt"))
        );
    }

    #[test]
    fn test_absolute_path_resolve_should_return_path_based_on_cwd_of_active_focused_window_in_kitty_env(
    ) {
        let result = AbsolutePath::resolve(
            &Path::new("file.txt"),
            &Environment::Kitty(test_fixture::LS_OUTPUT.clone()),
        )
        .unwrap();

        assert_eq!(
            result.buf,
            PathBuf::from(Path::new("/path/to/felis/file.txt"))
        );
    }
}

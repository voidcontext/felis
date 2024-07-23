# <p align="center"><code>kitty</code> + <code>helix</code> = <code>felis</code></p>

`felis` is the missing link between the `helix` editor and the `kitty` terminal: its purpose is to
simplify the integration between this two software.

## Commands

- open-file: opens a file in `helix`, by "typing" `ESC` + `:open path/to/file` + `ENTER` into the
  `kitty` window running `helix`.
  The command has an optional `--steel` switch, which is not going to type the full path into the
  editor, but write in a file then run the `felis-open` command. This command doesn't exist in
  `helix`, but can be added, if you're on the branch that adds the Steel integration. See the plugin
  section for more information.
- open-browser: runs the given file browser (e.g. [broot](https://github.com/Canop/broot)),
  optionally in a `kitty` window overlay on top of `helix`, then opens the selected file. This
  command also has a `--steel` option that uses helix' plugin system.

## Helix plugin

This is heavily experimental, and only works with a specific branch that adds a 
[steel](https://github.com/mattwparas/steel) based plugin system. The PR can be tracked 
[here](https://github.com/helix-editor/helix/pull/8675).

The helix plugin is exposed via the `default` package as a "passthru" attribute, and as standalone
package called `helix-plugin`. The result if this derivation is a single file and this file should
be symlinked into `~/.config/helix/`. After this the commands should just be wired into the main
config, e.g. in `helix.scm`:

```scheme
(define felis-path "@felis@")
(define broot-path "@broot@")

(require "felis.scm")
(provide felis-open
         file-browser
         file-browser-cwd)

(define (file-browser)
    (felis-file-browser felis-path broot-path))
(define (file-browser-cwd)
    (felis-file-browser-cwd felis-path broot-path))
```

_Please note: the actual paths needs to be substituted, e.g. with `pkgs.substituteAll` function._

## Why is it useful?

### Helix file explorer overlay

Running a file browser from `helix` gets as simple as this:

```toml
[keys.normal.space]
e = ":sh felis open-browser -l $(which broot)"
```

Notice that instead of just `broot`, the full path is used in this example, beause the program is
going to run a `kitty` overlay, where the shell environment is not initialised, so PATHs might be
missing (especially if you use `home-manager`).

To make this work, `broot` needs some configuration so that it prints the path and then exists when
a file is selected:

```toml
[[verbs]]
apply_to = "file"
internal = ":print_path"
invocation = "print_path"
key = "enter"
leave_broot = true
shortcut = "pp"
```

### Opening files from anywhere

Opening any selected file from `kitty` can be configured like this:

```conf
map ctrl+cmd+o pass_selection_to_program /path/to/felis/bin/felis open-file --context terminal
```

This is particularly useful when another program, e.g. a test runner prints file paths to the
standard output. Just select them with the mouse and open them in `helix`.

## How is felis trying to find the right helix instance to open the file?

If the path is relative, `felis` will try to determine the absolute path depending on the context.
In a "shell" context it is going to use the current directory (getcwd equivalent), in a "terminal"
context it lists `kitty` windows and tries to find the currently focused window and uses its
current working directory attribute. Once this is done, it is going to try to find a window which
runs `helix` (`../bin/hx`) and where the working directory is the same or the parent of the file's
directory (it doesn't have to be direct parent).

Let's see an example:

In window (1) the working directory is `/path/to/felis`, and `helix` is running. In window (2) the
working directory is the same, and we run some tests (so it is the focused active window), and the
output says there's an error in lib.rs on line 13, column 3. We select `src/lib.rs:13:3`, hit the
key combination that will pass this to `felis`, which in the end will run `felis open-file --context
terminal src/ lib.rs:13:3`. Based on the active focused window `felis` will resolve the absolute
path as `/path/to/felis/src.lib.rs:13:3`, it will find window (1) as it is running `helix` and its
working directory is the parent of the file. Once this window is found `felis` will send the key
sequence to open the file, and then it will focus the window.

## Roadmap

- [ ] declaratively (probably via TOML) define tab/window layout as a project environment, with
roles assigned to tabs/windows for easier scripting

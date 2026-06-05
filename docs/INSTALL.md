# Installing Zake

Zake is a small Rust command-line tool. Once it is built, the final `zake`
binary is portable across machines with the same operating system and CPU
architecture.

Your notes stay portable too: Zake stores ordinary Markdown files, YAML
frontmatter, and Git history in your notebook folder.

## Requirements

Zake itself is one binary, but a few external tools make the full workflow work:

- `git`: required for notebook history, status, staging, and commits.
- `rg` / ripgrep: required for `zake search` and TUI search.
- `$EDITOR`: optional, used by `zake open` and TUI `:open`.
- Rust toolchain: only required when installing from source.

Check the external tools:

```sh
git --version
rg --version
echo "$EDITOR"
```

If `$EDITOR` is empty, set it in your shell profile:

```sh
export EDITOR=nvim
```

Use `vim`, `nano`, `code --wait`, or any editor command you prefer.

## Recommended Install From Source

Install Rust first if you do not already have it:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then clone Zake and install the binary:

```sh
git clone https://github.com/OWNER/zake.git
cd zake
cargo install --path .
```

This places `zake` in Cargo's binary directory, usually:

```sh
~/.cargo/bin/zake
```

Make sure Cargo's binary directory is on your `PATH`:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Verify the install:

```sh
zake --version
zake --help
```

## Local Portable Binary

If you want a portable binary for yourself or another machine with the same OS
and CPU architecture:

```sh
cargo build --release
```

The compiled binary will be at:

```text
target/release/zake
```

Copy it to a directory on your `PATH`, for example:

```sh
mkdir -p "$HOME/.local/bin"
cp target/release/zake "$HOME/.local/bin/"
export PATH="$HOME/.local/bin:$PATH"
```

On Windows, the binary is:

```text
target\release\zake.exe
```

After copying the binary, the target machine does not need Rust installed. It
still needs `git` for Git features and `rg` for search.

## Installing Prebuilt Releases

When Zake publishes release binaries, installation should look like this:

1. Download the archive for your OS and CPU architecture.
2. Unpack it.
3. Move `zake` or `zake.exe` into a directory on your `PATH`.
4. Run `zake --help`.

Suggested release archive names:

```text
zake-aarch64-apple-darwin.tar.gz
zake-x86_64-apple-darwin.tar.gz
zake-x86_64-unknown-linux-gnu.tar.gz
zake-x86_64-pc-windows-msvc.zip
```

Suggested archive contents:

```text
zake
README.md
docs/INSTALL.md
```

## First Notebook

Create or enter a folder for your notes:

```sh
mkdir -p ~/notes
cd ~/notes
zake init
```

Create a note:

```sh
zake new "First Note"
```

Open the TUI:

```sh
zake
```

Run a health check:

```sh
zake doctor
```

## Updating

If you installed from source:

```sh
cd zake
git pull
cargo install --path .
```

If you installed a release binary, download the newer release and replace the old
binary.

## Uninstalling

If installed with Cargo:

```sh
cargo uninstall zake
```

If installed by copying a binary, remove it from the install directory:

```sh
rm "$HOME/.local/bin/zake"
```

## Making Zake More Portable For Releases

For maintainers, the simplest portable distribution is a release binary per
target platform. A good release workflow should:

- Build with `cargo build --release`.
- Run `cargo test` before packaging.
- Package one archive per target platform.
- Include `README.md` and `docs/INSTALL.md`.
- Document that `git` and `rg` are runtime dependencies.
- Avoid putting notebooks or generated cache files into release archives.

The notebook format is already portable because `.zake/config.toml` and notes are
plain files. The binary distribution is the only missing packaging layer.

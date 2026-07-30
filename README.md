# whoneeds

`whoneeds` shows all installed Arch Linux packages that directly or transitively depend on another installed package, grouped by whether they were explicitly or automatically installed.

It preserves the historical command interface:

```sh
whoneeds <package-name>
```

Example output:

```text
Explicitly installed packages that depend on [zlib]
  pacman
Other installed packages that depend on [zlib]
  curl
```

When no installed dependents are found, it prints:

```text
Packages that depend on [zlib]
  None
```

## Backend

`whoneeds` is implemented in safe Rust and delegates package graph discovery to official Arch tooling:

- `pactree -lru <package>` from `pacman-contrib` lists reverse dependencies.
- `pacman -Qqe` lists explicitly installed packages.

The queried package itself is removed from the reverse-dependency set. The remaining packages are sorted into an explicitly installed group and an automatically installed group. Empty groups are omitted.

## Development Checks

```sh
make check
```

The repository includes `.githooks/pre-commit`, which runs the same formatter, tests, and pedantic Clippy checks used by `make check`.

Enable the tracked hook in this clone with:

```sh
git config core.hooksPath .githooks
```

## Installation

```sh
make install DESTDIR="$pkgdir" PREFIX=/usr
```

Runtime dependency on Arch Linux:

- `pacman-contrib`

# whoneeds

`whoneeds` shows explicitly installed Arch Linux packages that transitively depend on another installed package.

It preserves the historical command interface:

```sh
whoneeds <package-name>
```

Example output:

```text
Packages that depend on [zlib]
  pacman
```

When no explicitly installed dependents are found, it prints:

```text
Packages that depend on [zlib]
  None
```

## Backend

`whoneeds` is implemented in safe Rust and delegates package graph discovery to official Arch tooling:

- `pactree -lru <package>` from `pacman-contrib` lists reverse dependencies.
- `pacman -Qqe` lists explicitly installed packages.

The result is the sorted intersection of those two sets, excluding the queried package itself.

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

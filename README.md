# Lūn

Lūn runs linters and formatters. Lūn is so fast that you can use it lint and
format on every keypress instead of just in a pre-commit hook or in CI.

## How to use it

Lūn requires a configuration file (usually `lun.toml`). An entry in this file
specifies a linter or formatter and which files it operates on. For example:

```toml
[[linter]]
cmd = "ruff check"
files = "*.py"
```

`lun init` can generate a configuration file for you.

Once you've got a configuration, just run `lun run`.

## Why it's so fast

Lūn is fast primarily because of three features:

1. Caching
2. Batched parallelism
3. Git awareness

Also, it's written in Rust.

### Caching

Whenever you run a linter on a file via Lūn, it saves a record (i.e., a hash)
constructed from  several components, including the file path, file metadata
(modification time, size, permissions, etc.), and the linter command line (see
the documentation for a comprehensive list). Before running any tool, Lūn first
consults this cache to see if it has a corresponding entry. If so, it skips
running the linter again.

### Batched parallelism

Lūn runs tools in parallel, but in *batches*. Tools usually have some nontrivial
start-up time. This might involve parsing a configuration file, but even just
`fork`/`exec` can take a while. Thus, Lūn doesn't just run one instance of each
tool for every changed file, but instead *batches* them. Given *n* files that
need to be linted and *c* cores, Lūn creates *c* size-balanced batches (*n*/*c*
files per batch if every file is the same size).

For the actual parallelism, Lūn utilizes [Rayon], or [Ninja] if `--ninja`
is passed.

[Rayon]: https://docs.rs/rayon/latest/rayon/
[Ninja]: https://ninja-build.org/

### Git awareness

You can configure Lūn to assume that files on specific Git [refs] (i.e.,
branches, commits, or tags) are already linted and formatted. For example, if
you run your linters in CI, you can safely assume that files in `origin/main`
are in good shape. Lūn will compare files to the known good refs, and only
lint and format changed files.

[refs]: https://git-scm.com/book/en/v2/Git-Internals-Git-References

## Commands

- `lun init`: create a new configuration file
- `lun run`: run formatters and linters
  - `--check`: run linters, run formatters in "check" mode (i.e., in CI)
  - `--format`: only run formatters
  - `--ninja`: use the Ninja backend
  - `--staged`: only run on staged files (i.e., in a pre-commit hook)
  - `--watch`: rerun when files are changed
- `lun add`: add a known tool to the configuration file
- `lun clean`: delete the cache

See `--help` for a comprehensive list.

## Comparison to other approaches

- Build systems based on file modification times like Make and Ninja can run
  linters incrementally and in parallel, but will always re-run them when
  files are changed even if you've linted those files before (see the "switched
  branches" benchmark below).
- lun is a bit like [lint-staged](https://github.com/lint-staged/lint-staged),
  but emphasizes caching and more advanced parallelism. It's also written in
  Rust.
- lun is like [qlty](https://github.com/qltysh/qlty), but far simpler.

## Benchmarks

The following benchmarks compare Lūn against [Ninja] by running `rustfmt` on
the `crates` subdirectory of the [Ruff] project.

[Ruff]: https://github.com/astral-sh/ruff

In the "clean checkout" scenario, we just run `rustfmt` against `crates/**/*.rs`
with an empty cache. We run Lūn in three configurations:

<!-- TODO: flag to assume origin/main is linted, add that here -->

- Default: just `lun run`
- No batching: `lun run --no-batch`
- Ninja: Uses `lun --dry-run --ninja --no-batch` to generate a Ninja
  configuration, then runs `ninja`. Reported times are just for `ninja` itself.

The results indicate that Lūn's batching algorithm significantly speeds up
linting.

| Scenario         | Configuration | Time  |
| ---------------- | ------------- | ----- |
| Clean checkout   | Default       | 2.0s  |
| Clean checkout   | Ninja         | 11.7s |
| Clean checkout   | No batching   | 11.6s |

In the "switched branches" scenario, we lint, checkout an old ref, don't make
any changes, and immediately go back to the current ref. The recorded time is
just for the second run.

This sort of scenario is problematic for `mtime`-based build systems like Make
or Ninja, but Lūn handles it easily due to its caching.

| Scenario          | Configuration | Time  |
| ----------------- | ------------- | ----- |
| Switched branches | Default       | 0.1s  |
| Switched branches | Ninja         | 12.4s |

You can reproduce these results like so:

```sh
git clone https://github.com/astral-sh/ruff
cd ruff
../lun/scripts/bench.sh
```

## Documentation

See the [documentation] for more information, such as how to install Lūn or
build it from source.

[documentation]: https://langston-barrett.github.io/lun/overview.html

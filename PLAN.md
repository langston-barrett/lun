# Config Refactoring Plan

## Overview

Refactor the configuration system to introduce a unified `Config` type that merges CLI arguments and disk configuration, replacing the current pattern where `Cli`, `Paths`, and `config::Config` are passed separately throughout the codebase.

## Current Architecture

```
main()
  ├─ Parse CLI: cli::Cli
  ├─ Init logging from cli.log
  ├─ Resolve paths: Paths::resolve(&cli)
  ├─ Load disk config: config::Config::load()
  └─ go(cli, &paths, config, out)
       └─ Dispatch based on cli.command
```

**Key types:**
- `cli::Cli` - Parsed CLI arguments
- `cli::Command` - Enum of subcommands (Run, Cache, Init, Add, Known, Warns)
- `Paths` - Resolved paths for config file, cache directory, cwd
- `config::Config` - Loaded from `lun.toml`
- `run::Config` (internal) - Merged config for run command specifically

## Target Architecture

```
main()
  ├─ Parse CLI: cli::Cli
  ├─ Init logging from cli.log (unchanged - stays separate)
  ├─ Construct Config::new(cli)  // loads disk config internally
  └─ go(config, out)
       └─ Dispatch based on config.command
```

**New types:**
- `Config` - Unified merged configuration (new, in `src/config.rs`)
- `DiskConfig` - Renamed from current `Config` (disk-loaded values only)
- `config::Command` - New enum mirroring `cli::Command` but with resolved configs
- `run::Config` - Constructed by `Config`

## Detailed Changes

### Phase 1: Rename existing types

1. **Rename `config::Config` to `config::DiskConfig`** (`src/config.rs`)
   - Update all imports and references
   - Keep the same structure and `load()` method
   - This is purely a rename with no logic changes

### Phase 2: Create the new unified `Config`

2. **Create new `Config` struct** (`src/config.rs`)

   ```rust
   pub(crate) struct Config {
       pub(crate) cache: PathBuf,
       pub(crate) command: Command,
       pub(crate) cwd: PathBuf,
       pub(crate) path: Option<PathBuf>,
       pub(crate) warns: Warns,
   }
   ```

3. **Create `config::Command` enum** (`src/config.rs`)

   ```rust
   pub(crate) enum Command {
       Add(Add),
       Cache(Cache),
       Init(Init),
       Known,
       Run(run::Config),
       Warns { warn: Option<String> },
   }
   ```

4. **Implement `Config::new(cli: Cli, log: LogOptions) -> Result<Self>`**

   This constructor:
   - Resolves paths (current `Paths::resolve` logic)
   - Loads disk config if path exists (current `DiskConfig::load` logic)
   - Merges `cli.warn` with `disk_config.warns` into `Warns`
   - Constructs the appropriate `Command` variant based on `cli.command`
   - For `Run`, calculates `show_progress: Format` from `log`, then constructs `run::Config`
   - For other commands, constructs their respective config types

   Note: `log` is used to calculate progress format only, not stored in `Config`.

### Phase 3: Create command-specific config types

5. **Update `run::Config`** (`src/run.rs`)
   - Make it `pub(crate)` so `config::Config` can construct it
   - Move `mk_config` logic into `run::Config::new()` or similar
   - Store file filtering parameters (`only_files`, `skip_files`, `staged`, `cwd`, `cache`) instead of pre-collected files
   - Add `pub(crate) fn collect_files(&self, out: &mut impl Write) -> Result<Vec<File>>` method
   - Constructor takes pre-calculated `show_progress: Format` (not `LogOptions`)

6. **Create `config::Cache`** (`src/config.rs`)

   ```rust
   pub(crate) struct Cache {
       pub(crate) command: CacheCommand,
   }

   pub(crate) enum CacheCommand {
       Rm,
       Gc { size: usize },
       Stats,
       Entry(EntryCommand),
   }
   ```

   This can reuse `cli::CacheEntryCommand` directly if its structure is sufficient.
   Note: `cache` path is already in `Config`, so `Cache` only needs the subcommand.

7. **Create `config::Init`** (`src/config.rs`)

   ```rust
   pub(crate) struct Init {
       pub(crate) options: cli::Init,  // reuse CLI struct if sufficient
   }
   ```

   Note: config path is already in `Config.path`, so `Init` only needs the options.

8. **Create `config::Add`** (`src/config.rs`)

   ```rust
   pub(crate) struct Add {
       pub(crate) options: cli::Add,  // reuse CLI struct if sufficient
   }
   ```

   Note: config path is already in `Config.path`, so `Add` only needs the options.

### Phase 4: Update the dispatch logic

9. **Update `go()` function** (`src/main.rs`)

   Change signature from:
   ```rust
   pub(crate) fn go(
       cli: cli::Cli,
       paths: &Paths,
       config: Option<config::Config>,
       out: &mut (impl Write + Send),
   ) -> Result<bool>
   ```

   To:
   ```rust
   pub(crate) fn go(
       config: &Config,
       out: &mut (impl Write + Send),
   ) -> Result<bool>
   ```

   Update dispatch to use `config.command` instead of `cli.command`.

10. **Update `main()`** (`src/main.rs`)

    ```rust
    fn main() -> Result<()> {
        let cli = cli::Cli::parse();
        let log = cli.log;  // Copy
        log::init_tracing(log);  // stays before Config
        let config = Config::new(cli, log)?;
        let ok = go(&config, &mut io::stderr())?;
        // ...
    }
    ```

### Phase 5: Update command handlers

11. **Update `run::go()`** (`src/run.rs`)

    Change signature from:
    ```rust
    pub(crate) fn go(
        cli: &cli::Cli,
        paths: &Paths,
        run_cli: &cli::Run,
        config: &config::Config,
        lints: &Warns,
        out: &mut (impl Write + Send),
    ) -> Result<RunResult>
    ```

    To:
    ```rust
    pub(crate) fn go(
        config: &run::Config,
        warns: &Warns,
        out: &mut (impl Write + Send),
    ) -> Result<RunResult>
    ```

    The `run::Config` contains all merged values. Remove `mk_config` call. Instead:
    - Call `config.collect_files(out)?` at the start of `go()`
    - Watch mode calls `collect_files()` on each iteration

12. **Update `init::go()`** (`src/init.rs`)

    ```rust
    pub(crate) fn go(config_path: &Path, init: &config::Init) -> Result<()>
    ```

    Or if `config::Init` just wraps `cli::Init`, could pass that directly.

13. **Update `add::go()`** (`src/add.rs`)

    ```rust
    pub(crate) fn go(config_path: &Path, add: &config::Add) -> Result<()>
    ```

    Or if `config::Add` just wraps `cli::Add`, could pass that directly.

14. **Update cache operations** (`src/main.rs` dispatch + cache module)

    Pass `config::Cache` to cache operations (along with `config.cache` path).

### Phase 6: Remove deprecated types

15. **Remove `Paths` struct** (`src/main.rs`)
    - Its fields are now in `Config`
    - Remove `Paths::resolve()` - logic moves to `Config::new()`

### Phase 7: Handle watch mode

16. **Update watch mode** (`src/run.rs`)

    Watch mode takes `&run::Config` and `&Warns`. On each iteration:
    - Calls `config.collect_files(out)?` to get fresh file list
    - Runs the tools on those files

    Since `run::Config` stores file filtering parameters (not pre-collected files), watch mode works naturally.

## Design Decisions

### Watch mode file re-collection

`run::Config` stores file filtering parameters (`only_files`, `skip_files`, `staged`) and has a `collect_files(&self) -> Result<Vec<File>>` method. This allows watch mode to re-collect files on each iteration without reconstructing the entire config.

### Progress format

`Config::new()` calculates `show_progress: Format` from `LogOptions` and passes it to `run::Config::new()`. `LogOptions` is not stored in `Config`.

### Error handling

`Config::new()` returns `Result<Config>`. File collection is deferred to `run::go()` (via `run::Config::collect_files()`), so `Config::new()` only does disk config loading and warns merging.

### Command config locations

Keep `run::Config` in `run.rs` (complex and module-specific). Put simpler ones (`Cache`, `Init`, `Add`) in `config.rs` since they're mostly wrappers.

## File Change Summary

| File | Changes |
|------|---------|
| `src/config.rs` | Rename `Config` → `DiskConfig`, add new `Config`, `Command`, `Cache`, `Init`, `Add` |
| `src/main.rs` | Remove `Paths`, update `go()` signature, update `main()` |
| `src/run.rs` | Make `Config` public, add `collect_files()` method, update `go()` signature |
| `src/cli.rs` | No structural changes (types reused where sufficient) |
| `src/init.rs` | Update `go()` signature |
| `src/add.rs` | Update `go()` signature |
| `src/warn/warns.rs` | No changes (already has `from_cli_and_config`) |

## Migration Strategy

1. Do the rename (`Config` → `DiskConfig`) first as a separate commit
2. Add new types without removing old ones
3. Update `go()` and handlers incrementally
4. Remove `Paths` once nothing uses it
5. Clean up any remaining references

This allows for incremental, testable changes rather than one large refactor.

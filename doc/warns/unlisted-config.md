# `unlisted-config`

Warns when `lun` finds a configuration file for a tool that is not listed in
`lun.toml`. Tools might have different output in different configurations, and
so it is important that they are re-run if their configuration changes. Tool
configuration files listed in `lun.toml` form part of `lun`'s cache keys.

Default level: `allow`

In groups:

- `all`
- `pedantic`

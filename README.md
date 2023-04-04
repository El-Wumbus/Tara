<div align="center">

# Tara

[![crates.io][crates.io-badge]][crates.io]
[![github-release][github-release-badge]][github-release]
[![github-license][github-license-badge]][github-license]

Tara is a new, free, open-source self-hostable, Discord bot.

Tara works on Linux (with macOS and Windows support coming soon).

[Installation](#installation) • [Using](#using)

</div>

# Installation

Tara can be installed in 2 simple steps:

1. **Install executable**

If your desired platform isn't seen below, please [open an issue][issues].

<details>
<summary>Linux</summary>

> The recommended way to install Tara is by way of a package manager.
> If using `cargo install`, some dependencies won't automatically be installed. You'll need to install `sqlite3` previous to running the instructions.
> On Debian and Ubuntu systems the required package is `libsqlite3-dev`, on Arch and related systems it's `sqlite`.
>
> | Distribution | Repository      | Instructions                  |
> | ------------ | --------------- | ----------------------------- |
> | *Any*        | **[crates.io]** | `cargo install tara --locked` |

</details>

2. **Configure**

Before the bot can be started successfully, it needs to be configured.

<details>
<summary>Linux</summary>

> The configuration file is located at `/etc/tara.d/tara.toml`.

</details>

The configuration file should look similarly to below:

```toml
direct_message_cooldown = 5 # Optional

[secrets]
# Discord bot token
token = "<DISCORD_TOKEN>"

# API key from currencyapi.com.
currencyApiKey = "<CURRENCYAPI.COM>" # Optional
```

Present in all the configuration files are the following keys:

- *`direct_message_cooldown`* - This optional key is to set the minimum duration, in seconds, to allow between running commands in a direct message. The default is `3`.

- *`secrets.token`* - The discord token can be aquired according to *[Building your first Discord app][discord-getting-started]*.

- *`secrets.currencyApiKey`* - The `currencyApiKey` is an optional key to enable the currency conversion feature. This can be aquired from [currencyapi.com][currencyapi]. The feature will, at most, refresh every six hours. This means the feature will never need a paid API key.

# Using

## Commands

| Name                      | Description                                                                              | Usable in  DMs | Permissions  |
| ------------------------- | ---------------------------------------------------------------------------------------- | -------------- | ------------ |
| `define`                  | Defines an English word                                                                  | Yes            | *NONE*       |
| `wiki`                    | Searches for a wikipedia page and returns a summary                                      | Yes            | *NONE*       |
| `random coin`             | Flips a coin                                                                             | Yes            | *NONE*       |
| `random cat`              | Gives a random cat photo                                                                 | Yes            | *NONE*       |
| `random dog`              | Gives a random dog photo                                                                 | Yes            | *NONE*       |
| `random quote`            | Gives a random quote                                                                     | Yes            | *NONE*       |
| `random number`           | Generates a random number between optional low and high bounds (inclusive)               | Yes            | *NONE*       |
| `search duckduckgo`       | Search *[DuckDuckGo][duckduckgo]* for a search term. Results are censored.               | Yes            | *NONE*       |
| `conversions temperature` | Convert one temperature unit to another. Supports celsius, kelvin, and fahrenheit        | Yes            | *NONE*       |
| `conversions currency`    | Convert from one currency to another. Only enabled when `secrets.currencyApiKey` is set. | Yes            | *NONE*       |
| `settings set *`          | Set settings for the current guild                                                       | No             | MANAGE_GUILD |
| `settings view *`         | See current guild settings                                                               | No             | MANAGE_GUILD |
| `role add`                | Give yourself a self-assignable role                                                     | No             | *NONE*       |
| `role remove`             | Remove a self-assignable role                                                            | No             | *NONE*       |
| `role list`               | List all self-assignable roles                                                           | No             | *NONE*       |

[crates.io]: https://crates.io/crates/tara
[crates.io-badge]: https://img.shields.io/crates/v/tara?logo=Rust&style=flat-square
[github-license]: https://github.com/El-Wumbus/Tara/blob/master/LICENSE
[github-license-badge]: https://img.shields.io/github/license/El-Wumbus/Tara?logo=Apache&style=flat-square
[github-release]: https://github.com/El-Wumbus/Tara/releases/latest
[github-release-badge]: https://img.shields.io/github/v/release/El-Wumbus/Tara?logo=GitHub&style=flat-square
[issues]: https://github.com/El-Wumbus/Tara/issues/new
[discord-getting-started]: https://discord.com/developers/docs/getting-started
[currencyapi]: https://currencyapi.com/
[duckduckgo]: https://duckduckgo.com/html

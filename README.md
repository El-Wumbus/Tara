<div align="center">

# Tara

[![crates.io][crates.io-badge]][crates.io]
[![github-release][github-release-badge]][github-release]
[![github-license][github-license-badge]][github-license]

Tara is a modern, free, open-source, self-hostable Discord bot.

Tara works on Linux and macOS.

[Installation](#installation) • [Using](#using)

</div>

# Installation

Tara can be installed in 2 simple steps:

1. **Install executable**

If your desired platform isn't seen below, please [open an issue][issues].

<details>
<summary>Linux</summary>

> The recommended way to install Tara is by way of a package manager, however, **[crates.io]** [may be outdated](https://github.com/El-Wumbus/Tara/pull/3) and Tara should be installed from [GitHub releases][github-release].
> If using `cargo install`, some dependencies won't automatically be installed. You'll need to install `sqlite3` previous to running the instructions.
> On Debian and Ubuntu systems the required package is `libsqlite3-dev`, on Arch and related systems it's `sqlite`.
>
> | Distribution | Repository      | Instructions                  |
> | ------------ | --------------- | ----------------------------- |
> | *Any*        | **[crates.io]** | `cargo install tara --locked` |

</details>

<details>
<summary>macOS</summary>

> The recommended way to install Tara is by way of a package manager, however, **[crates.io]** [may be outdated](https://github.com/El-Wumbus/Tara/pull/3) and Tara should be installed from [GitHub releases][github-release].
> | Repository      | Instructions                 |
> | --------------- | ---------------------------- |
> | **[crates.io]** | `cargo install tara --locked`|

</details>

2. **Configure**

Before the bot can be started successfully, it needs to be configured.
Tara has an interactive setup subcommand, `tara config init`.

```sh
$ tara config init --help
tara-config-init 0.3.1
Create configuration files with a user-provided configuration

USAGE:
    tara config init

FLAGS:
    -h, --help    Prints help information
```

`tara config init` will create a configuration file in the appropriate location. If this needs to
be modified it can be.
The file's content's should be the same regardless of operating system, but the location in the file system will be different.

<details>
<summary>Linux</summary>

> Tara looks for a configuration file in this order:
>
> 1. `$XDG_CONFIG_HOME/Tara/tara.toml` or `$HOME/.config/Tara/tara.toml`
> 2. `/etc/tara.d/tara.toml`

</details>

<details>
<summary>macOS</summary>

> Tara's configuration file is located here: `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/tara.toml`

</details>

The configuration file should look similarly to below:

```toml
randomErrorMessage = false

[secrets]
# Discord bot token
token = "<DISCORD_TOKEN>"

# API key from currencyapi.com.
currencyApiKey = "<CURRENCYAPI.COM>" # Optional
```

All accepted keys:

- *`randomErrorMessage`* - This key allows for error messages to be selected randomly from a set loaded from a JSON document.
  If setting this key to `true`, it will look in the default locations for a `error_messages.json` file. If enabled and the file
  cannot be parsed (because it doesn't exist or is invalid), Tara will continue with the default error messages. Another choice
  is to set this to the path of the error messages file. A value of `false` will use a singular, static error message.

  The default location for the error messages file is system dependant.

  <details>
  <summary>Linux</summary>

    > Tara will look in these locations for an existing file.
    >
    > 1. `$XDG_CONFIG_HOME/Tara/error_messages.json` or `$HOME/.config/Tara/error_messages.json`
    > 2. `/etc/tara.d/error_messages.json`

  </details>

  <details>
  <summary>macOS</summary>

    > Tara will look here for an existing file:
    > `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/error_messages.json`

  </details>

  

- *`direct_message_cooldown`* - This optional key is to set the minimum duration, in seconds, to allow between running commands in a direct message. The default is `3`.

- *`secrets.token`* - The discord token can be aquired according to *[Building your first Discord app][discord-getting-started]*.

- *`secrets.currencyApiKey`* - The `currencyApiKey` is an optional key to enable the currency conversion feature. This can be aquired from [currencyapi.com][currencyapi]. The feature will, at most, refresh every six hours. This means the feature will never need a paid API key.

# Using

## Running

To start Tara, use the `tara daemon` command. If no errors or warnings occur, Tara's stdout and stderr will be blank. If Tara has a proper Discord token, then it's [ready to use](#discord-commands).

```sh
$ tara daemon --help
tara-daemon  0.3.1
Start Tara

USAGE:
    tara daemon [OPTIONS]

FLAGS:
    -h, --help    Prints help information

OPTIONS:
        --config <config>    Specify a configuration file to use instead of the default
```

## Discord Commands

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

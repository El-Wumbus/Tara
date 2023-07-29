<div align="center">

# Tara

[![github-release][github-release-badge]][github-release]
[![AUR][aur-badge]][AUR]
[![crates.io][crates.io-badge]][crates.io]
[![github-license][github-license-badge]][github-license]

Tara is a modern, free, open-source, self-hostable Discord bot.

Tara works on Linux and macOS.

[Installation](#installation) • [Using](#using)

</div>

## Installation

Tara can be installed very easily on [Linux](#linux) or [macOS](#macos).  
If your desired platform isn't available, please [open an issue][issues].

### Linux

The recommended way to install Tara is by way of a package manager.
:warning: **[crates.io]** [is very outdated](https://github.com/El-Wumbus/Tara/pull/3)
and Tara should be installed from an alternative source like [GitHub releases][github-release] instead.  

| Distribution | Repository      | Instructions                  |
| ------------ | --------------- | ----------------------------- |
| *Any*        | **[crates.io]** | `cargo install tara --locked` |
| *Arch Linux* | **[AUR]**       | `yay -S tara`                 |

When installing from the **[AUR]** `yay` is the helper used in the instructions,
but one isn't required or an alternative like `paru` may be used.

Tara looks for a configuration file in this order:
1. `$XDG_CONFIG_HOME/tara/tara.toml` or `$HOME/.config/tara/tara.toml`
2. `/etc/tara.d/tara.toml`

Now get to [configuring Tara](#configuration).

### macOS

The recommended way to install Tara is by way of a package manager.
:warning: **[crates.io]** [is very outdated](https://github.com/El-Wumbus/Tara/pull/3)
and Tara should be installed from an alternative source like [GitHub releases][github-release] instead.

| Repository                | Instructions                  |
| ------------------------- | ----------------------------- |
| :warning: **[crates.io]** | `cargo install tara --locked` |

Tara's configuration file is located here: `$HOME/Library/Application Support/com.github.El-Wumbus.Tara/tara.toml` on macOS.

Now get to [configuring Tara](#configuration).

## Configuration

Before running Tara you must configure it.
The configuration file should look similarly to below:

```toml
randomErrorMessage = false

[secrets]
# Discord bot token
token = "<DISCORD_TOKEN>"
# For currency conversions
currencyApiKey = "<CURRENCYAPI.COM>" # Optional
# For image search and random images
unsplash_key = "<FROM UNSPLASH.COM>" # Optional
# For movie and series metadata (If omitted default ones will be used)
omdb_api_key = "<FROM OMDBAPI.com>" # Optional

[music] # Optional
enabled = false
```

More notes on the above noted configurations:

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

- *`secrets.token`* - The discord token can be aquired according to *[Building your first Discord app][discord-getting-started]*.

- *`secrets.currencyApiKey`* - The `currencyApiKey` is an optional key to enable the currency conversion feature. This can be aquired from [currencyapi.com][currencyapi]. The feature will, at most, refresh every six hours. This means the feature will never need a paid API key.

- *`music`* - Optional: This only takes effect if Tara is compiled with the alpha feature `music` enabled.
  - *`music.enabled`* - Enables or disables the music feature at runtime.

## Using

### Running

To start Tara, use the `tara` command. You should expect logged output.
If Tara has a proper Discord token, then it's [ready to use](#discord-commands).  

Provided for Linux users who use Systemd is a [`extra/tara.service`](extra/tara.service)
file that can be used to run Tara.

```
$ tara --help
Tara 6.0.0
Decator <decator.c@proton.me>
Tara is a modern, free, open-source, self-hostable Discord bot.

USAGE:
    tara [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -l, --log-level <LOGLEVEL>    
        --config <config>         Specify a configuration file to use instead of the default
```

## Discord Commands

| Name                      | Description                                                                                | Usable in  DMs | Permissions  |
| ------------------------- | ------------------------------------------------------------------------------------------ | -------------- | ------------ |
| `define`                  | Defines an English word                                                                    | Yes            | *NONE*       |
| `wiki`                    | Searches for a wikipedia page and returns a summary                                        | Yes            | *NONE*       |
| `random coin`             | Flips a coin                                                                               | Yes            | *NONE*       |
| `random cat`              | Gives a random cat photo                                                                   | Yes            | *NONE*       |
| `random dog`              | Gives a random dog photo                                                                   | Yes            | *NONE*       |
| `random quote`            | Gives a random quote                                                                       | Yes            | *NONE*       |
| `random number`           | Generates a random number between optional low and high bounds (inclusive)                 | Yes            | *NONE*       |
| `random image`            | Get a random image                                                                         | Yes            | *NONE*       |
| `random emoji`            | Get a random Emoji                                                                         | Yes            | *NONE*       |
| `random fact`             | Get a random fun fact                                                                      | Yes            | *NONE*       |
| `search duckduckgo`       | Search *[DuckDuckGo][duckduckgo]* for a search term. Results are censored.                 | Yes            | *NONE*       |
| `search image`            | Search for an image from the internet                                                      | Yes            | *NONE*       |
| `conversions temperature` | Convert one temperature unit to another. Supports celsius, kelvin, and fahrenheit          | Yes            | *NONE*       |
| `conversions currency`    | Convert from one currency to another. (Only enabled when `secrets.currencyApiKey` is set.) | Yes            | *NONE*       |
| `movie`                   | Get information about a movie                                                              | Yes            | *NONE*       |
| `series`                  | Get information about a TV series                                                          | Yes            | *NONE*       |
| `settings set *`          | Set settings for the current guild                                                         | No             | MANAGE_GUILD |
| `settings view *`         | See current guild settings                                                                 | No             | MANAGE_GUILD |
| `role add`                | Give yourself a self-assignable role                                                       | No             | *NONE*       |
| `role remove`             | Remove a self-assignable role                                                              | No             | *NONE*       |
| `role list`               | List all self-assignable roles                                                             | No             | *NONE*       |
| `music play`              | Join your voice channel and play a song [from YouTube]                                     | No             | *NONE*       |
| `music stop`              | Stop playback                                                                              | No             | *NONE*       |
| `music pause`             | Pause the currently playing track                                                          | No             | *NONE*       |
| `music unpause`           | Resume a currently paused track                                                            | No             | *NONE*       |
| `music leave`             | Leave your voice channel                                                                   | No             | *NONE*       |

[crates.io]: https://crates.io/crates/tara
[AUR]: https://aur.archlinux.org/packages/tara
[aur-badge]: https://img.shields.io/aur/version/tara?label=AUR&style=flat-square
[crates.io-badge]: https://img.shields.io/crates/v/tara?logo=Rust&style=flat-square
[github-license]: https://github.com/El-Wumbus/Tara/blob/master/LICENSE
[github-license-badge]: https://img.shields.io/github/license/El-Wumbus/Tara?logo=Apache&style=flat-square
[github-release]: https://github.com/El-Wumbus/Tara/releases/latest
[github-release-badge]: https://img.shields.io/github/v/release/El-Wumbus/Tara?logo=GitHub&style=flat-square
[issues]: https://github.com/El-Wumbus/Tara/issues/new
[discord-getting-started]: https://discord.com/developers/docs/getting-started
[currencyapi]: https://currencyapi.com/
[duckduckgo]: https://duckduckgo.com/html

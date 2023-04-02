# TARA

A free, open-source, community-driven, speedy, stable, self-hosted, Discord bot.

# Dependencies

- Sqlite3

## Build Dependencies

- [Cargo (Rust)](https://www.rust-lang.org/tools/install)

## Debian, Ubuntu, etc.

```bash
sudo apt install libsqlite3-dev -y
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

```
After installing rustup (line 2), run `rustup default nightly`.
Then, compile and install Tara.

## Arch, Manjaro, etc.

```bash
sudo pacman -S sqlite
```

# Installation

Currently, Tara only supports Linux (Though this will change).

## With Cargo install

If using this method, the provided systemd service file `extra/tara.service` will have to be modified.

```bash
cargo install tara
```

## Manually

```bash
git clone https://github.com/El-Wumbus/Tara
cd Tara
cargo build --release
sudo install -Dvm755 target/release/tara /usr/local/bin/tara
sudo install -Dvm754 extra/tara.service /etc/systemd/system/tara.service
```

To enable and start the service.

```bash
sudo systemctl enable --now tara
```
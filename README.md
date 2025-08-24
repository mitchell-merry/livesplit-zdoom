# livesplit-zdoom

messing around with reflection in zdoom games

tested with Dismantled, an LZDoom 3.82 game

# MEGA DISCLAIMER
I am bad at Rust

This is a hard language

## Compilation

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler: [Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:
```sh
rustup target add wasm32-wasip1 --toolchain nightly
```

The autosplitters can now be compiled:
```sh
cargo b --release
```

The autosplitters are then available at:
```
target/wasm32-wasip1/release/<name>.wasm
```

Make sure to look into the [API documentation](https://livesplit.org/asr/asr/) for the `asr` crate.

## Development

You can use the [debugger](https://github.com/LiveSplit/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory, step through the code and more.

`cargo build` will build all the autosplitters. Specify `-p <package>` to only compile
a specific autosplitter.

# fox32

![fox32 logo](docs/logos/fox32.png)

**fox32** (stylized in all lowercase) is a 32 bit fantasy computer architecture, with a custom operating system and user interface inspired by various classic computers.

![Screenshot of fox32rom](docs/screenshots/screenshot_fox32rom.png)

## Getting Started

**Note: This software is still very much in an early stage, and is currently more focused towards developers rather than end-users.**

Prebuilt binaries are available on the [GitHub Actions page](https://github.com/fox32-arch/fox32/actions).

### Building

Simply run `cargo build --release`. The resulting binary will be saved as `target/release/fox32`. You can also run `cargo run --release` if you want to run it directly.

### Usage

**fox32** attempts to read its ROM (called **fox32rom**) from a file called `fox32.rom` in the current directory. If this file isn't found then it falls back to `../fox32rom/fox32.rom`, and if this file isn't found then it exits. **fox32rom** can be found [here](https://github.com/fox32-arch/fox32rom).

Passing files as arguments will mount those files as disks, in the order that the arguments were passed.

See [encoding.md](docs/encoding.md) for information about the instruction set.

## License
This project is licensed under the [MIT license](LICENSE).

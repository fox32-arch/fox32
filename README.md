# fox32

![fox32 logo](docs/logos/fox32.png)

**fox32** (stylized in all lowercase) is a 32 bit fantasy computer architecture, with a custom operating system and user interface inspired by various classic computers.

![Screenshot of fox32rom](docs/screenshots/screenshot_fox32rom.png)

## Getting Started

**Note: This software is still very much in an early stage, and is currently more focused towards developers rather than end-users.**

### Building

Clone this repository, `cd` into it, then clone the [fox32core](https://github.com/fox32-arch/fox32core) repository in a folder called `fox32core`.

After that, just run `cargo build --release`. The resulting binary will be saved as `target/release/fox32`

### Usage

**fox32** attempts to read its ROM from a file called `fox32.rom` in the current directory. If this file isn't found then it falls back to `../fox32rom/fox32.rom`, and if this file isn't found then it exits.

Passing a file as an argument will mount that file as Disk 0.

See [encoding.md](encoding.md) for information about the instruction set.

## License
This project is licensed under the [MIT license](LICENSE).

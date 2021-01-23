![235 - hockey results with a familiar feel](docs/nhl-235-banner.png)

# nhl-235

NHL results on your command-line with a familiar feel.

For decades, number 235 has been an important part of the morning routine of Finnish hockey fans. YLE's (Finnish Broadcasting Company) teletext page 235 displays on-going or latest results for NHL games. Its cultural importance is so big that I wanted to pay homage to it with this project.

## Install

### Cargo

You can either install via [Rust's Cargo](https://crates.io):

```
cargo install nhl-235
```

I recommend adding a symlink to have a traditional `235` feeling:

```
ln -s ~/.cargo/bin/nhl-235 /usr/local/bin/235
```

If you're storing your cargo packages in a different folder, replace `~/.cargo/bin/nhl-235` with your folder path.

### Download binaries

or [download the latest binaries from GitHub](https://github.com/Hamatti/nhl-235/releases/latest).

Store the file with filename `235` in a folder that is in the path.

## Usage

### Basic usage

```
235
```

### Current version

```
235 --version
```

## License

This project is [licensed under the MIT License](LICENSE)

## Acknowledgements

### nhl-score-api

This project uses [peruukki/nhl-score-api](https://github.com/peruukki/nhl-score-api) for the data.

### Futurice Spice Program

Development of 235 has been a grateful recipient of the [Futurice Open Source sponsorship program](https://spiceprogram.org).

### YLE Tekstitv

None of this would exist without the cultural importance of [YLE's teletext](https://yle.fi/aihe/tekstitv) and the page 235 has had in Finnish hockey fan culture.

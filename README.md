# getlrc

A fast, concurrent TUI application for fetching and storing synchronized lyrics from [lrclib.net](https://lrclib.net).

## Features

- 🎵 **Multi-format Support** - FLAC, Opus, MP3, AAC, OGG, APE, WAV, M4A
- 🚀 **Concurrent Processing** - Async worker with rate-limited API calls
- 💾 **Smart Caching** - SQLite-backed negative cache prevents redundant lookups
- 🎨 **Beautiful TUI** - Real-time progress tracking with color-coded status
- 📁 **XDG Compliant** - Follows Linux standards (`~/.local/share/getlrc`)
- ⚡ **60fps UI** - Non-blocking event loop maintains responsiveness

## Installation

### Quick Install (Recommended)

```bash
# Clone the repository
git clone https://github.com/c0mpile/getlrc.git
cd getlrc

# Run the installation script
./install.sh
```

The script will:
1. Build the release binary
2. Install to `~/.local/bin/getlrc`
3. Check if `~/.local/bin` is in your `$PATH`

### Manual Installation

```bash
# Build release binary
cargo build --release

# Copy to local bin
cp target/release/getlrc ~/.local/bin/

# Or use the built-in installer
cargo run --release -- install
```

### Add to PATH

If `~/.local/bin` is not in your PATH, add this to your shell config:

**Bash** (`~/.bashrc`):
```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Zsh** (`~/.zshrc`):
```bash
export PATH="$HOME/.local/bin:$PATH"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
set -gx PATH $HOME/.local/bin $PATH
```

Then reload your shell:
```bash
source ~/.bashrc  # or ~/.zshrc
```

## Usage

```bash
# Scan a music directory
getlrc ~/Music

# Scan a specific album
getlrc ~/Music/Artist/Album

# Show help
getlrc --help

# Uninstall
getlrc uninstall
```

## How It Works

1. **Scan** - Recursively finds all audio files in the target directory
2. **Skip** - Ignores files that already have `.lrc` sidecars
3. **Extract** - Reads metadata (artist, title, album, duration) using `lofty`
4. **Cache Check** - Queries local SQLite database for previously unfound tracks
5. **API Call** - Fetches lyrics from lrclib.net (rate-limited to 10 req/s)
6. **Write** - Saves synchronized lyrics as `.lrc` files next to audio files

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    TUI (Ratatui)                        │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ Header  │  │ Progress │  │   Logs   │  │ Footer  │ │
│  └─────────┘  └──────────┘  └──────────┘  └─────────┘ │
└───────────────────┬─────────────────────────────────────┘
                    │ mpsc::unbounded_channel
                    ▼
┌─────────────────────────────────────────────────────────┐
│                   Worker (Tokio)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│  │ Scanner  │→ │  Cache   │→ │ LRCLIB   │→ │  .lrc  │ │
│  │ (walkdir)│  │ (SQLite) │  │   API    │  │ Writer │ │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────┘
```

## Data Storage

All persistent data follows XDG Base Directory specification:

- **Cache Database**: `~/.local/share/getlrc/negative_cache.db`
- **Debug Logs**: `~/.local/share/getlrc/logs/getlrc.log`

### Logging

All debug output (API calls, metadata extraction, errors) is written to the log file to keep the TUI clean. The log file includes:
- Full file paths
- API request URLs
- Detailed error messages
- Timing information

To view logs in real-time:
```bash
tail -f ~/.local/share/getlrc/logs/getlrc.log
```

## UI Log Legend

The TUI displays concise status updates:

- **[✓]** - Lyrics downloaded and saved
- **[✗]** - Lyrics not found on lrclib.net
- **[~]** - Cached (previously not found, skipped API call)
- **[○]** - Already has .lrc file (skipped)
- **[!]** - Error occurred (see log file for details)

## Progress Bar Legend

- **Green (█)** - Lyrics downloaded from API
- **Yellow (█)** - Cached (previously not found)
- **Blue (█)** - Already has .lrc file (skipped)
- **Dark Gray (░)** - Not yet processed

## Dependencies

- `tokio` - Async runtime
- `ratatui` - Terminal UI framework
- `lofty` - Audio metadata extraction
- `rusqlite` - SQLite database
- `reqwest` - HTTP client
- `walkdir` - Directory traversal

## Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt
```

## License

MIT

## Credits

- Lyrics provided by [lrclib.net](https://lrclib.net)
- Built with Rust 🦀

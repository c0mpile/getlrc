# getlrc

A fast, interactive TUI application for fetching and storing synchronized lyrics from [lrclib.net](https://lrclib.net) with atomic session persistence and smart caching.

## ✨ Features

### Core Functionality
- 🎵 **Multi-format Support** - FLAC, Opus, MP3, AAC, OGG, APE, WAV, M4A
- 🚀 **Async Processing** - Non-blocking worker with rate-limited API calls (10 req/s)
- 💾 **Smart Caching** - SQLite-backed negative cache prevents redundant lookups
- 📦 **Atomic Session Persistence** - Resume interrupted scans from exactly where you left off
- 🎨 **Beautiful TUI** - Real-time progress tracking with color-coded status
- 📁 **XDG Compliant** - Follows Linux standards (`~/.local/share/getlrc/`)

### Interactive Controls
- ⏸️ **Pause/Resume** - Pause processing at any time, resume later
- 💾 **Auto-save on Pause** - Session state saved atomically when paused
- 🔄 **Session Recovery** - Automatically resumes from saved sessions
- 📊 **Real-time Progress** - Live progress bar with 100% completion guarantee
- 📜 **Auto-scrolling Logs** - Latest entries always visible

### Reliability Features
- ⚛️ **Atomic Saves** - Crash-safe session persistence using temp files
- 🔍 **Integrity Checks** - Validates session files before resuming
- 🛡️ **Permission Verification** - Checks write access before starting
- 📝 **Comprehensive Logging** - All operations logged to file for debugging
- 🎯 **100% Progress Accuracy** - Progress bar always reaches completion

## 📦 Installation

### Using Cargo (Recommended)

```bash
# Clone the repository
git clone https://github.com/c0mpile/getlrc.git
cd getlrc

# Install locally
cargo install --path .
```

The binary will be installed to `~/.cargo/bin/getlrc` (or `~/.local/bin` if configured).

### Verify Installation

```bash
# Check if installed
which getlrc

# Verify PATH (getlrc will warn if ~/.local/bin is missing)
echo $PATH | grep -o "$HOME/.local/bin"
```

### Add to PATH (if needed)

If `~/.cargo/bin` or `~/.local/bin` is not in your PATH:

**Bash/Zsh** (`~/.bashrc` or `~/.zshrc`):
```bash
export PATH="$HOME/.cargo/bin:$PATH"
# or
export PATH="$HOME/.local/bin:$PATH"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
set -gx PATH $HOME/.cargo/bin $PATH
```

Then reload your shell:
```bash
source ~/.bashrc  # or ~/.zshrc
```

## 🚀 Usage

### Basic Usage

```bash
# Scan a music directory
getlrc ~/Music

# Scan a specific album
getlrc ~/Music/Artist/Album

# Show help
getlrc --help
```

### Interactive Controls

While the TUI is running:

| Key | Action | Description |
|-----|--------|-------------|
| `q` | Quit | Exit application (saves session if paused) |
| `p` | Pause | Pause processing and save session |
| `r` | Resume | Resume processing from paused state |

**Note**: When you press `p` (Pause), the current session is automatically saved. You can safely quit with `q` and resume later by running `getlrc` again on the same directory.

### Session Persistence

**Pause and Resume Workflow**:
```bash
# Start scanning
$ getlrc ~/Music
# Processing: 45/200 files...

# Press 'p' to pause
# Session saved to ~/.local/share/getlrc/session.json

# Press 'q' to quit
# Application exits, state preserved

# Later, resume the scan
$ getlrc ~/Music
📂 Resuming previous session...
# Continues from file 46/200 with full log history
```

**Session Features**:
- ✅ Atomic saves (crash-safe)
- ✅ Integrity checks (detects stale sessions)
- ✅ Log history restoration (visual continuity)
- ✅ Progress count preservation
- ✅ Automatic cleanup on completion

## 📊 UI Reference

### Status Symbols

The TUI displays concise status updates for each file:

| Symbol | Meaning | Description |
|--------|---------|-------------|
| `[✓]` | Downloaded | Lyrics downloaded and saved successfully |
| `[~]` | Cached | Previously not found, skipped API call |
| `[○]` | Existing | Already has .lrc file, skipped |
| `[✗]` | Not Found | Lyrics not available on lrclib.net |
| `[!]` | Error | Processing error (see logs for details) |

### Progress Bar Colors

| Color | Meaning |
|-------|---------|
| 🟢 Green | Lyrics downloaded from API |
| 🟡 Yellow | Cached (previously not found) |
| 🔵 Blue | Already has .lrc file |
| ⚫ Dark Gray | Not yet processed |

### TUI Layout

```
┌─────────────────────────────────────────────────────────┐
│ getlrc - Processing...                                  │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ Progress                                                │
│ ████████████████████████████████████████████████░░░░░░░ │ 80%
│ ● Downloaded: 45  ● Cached: 12  ● Existing: 8           │
└─────────────────────────────────────────────────────────┘
[✓] Downloaded | [~] Cached | [○] Existing | [✗] Not Found | [!] Error
┌─────────────────────────────────────────────────────────┐
│ Logs                                                    │
│ [✓] song1.mp3                                           │
│ [~] song2.flac                                          │
│ [○] song3.m4a                                           │
│ ... (auto-scrolls to latest)                            │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│ q Quit | p Pause                                        │
└─────────────────────────────────────────────────────────┘
```

## 🔧 How It Works

### Processing Pipeline

1. **Environment Verification** - Checks directories and permissions
2. **Session Check** - Looks for existing session to resume
3. **Directory Scan** - Recursively finds all audio files
4. **Skip Existing** - Ignores files that already have `.lrc` sidecars
5. **Metadata Extraction** - Reads artist, title, album, duration using `lofty`
6. **Cache Lookup** - Checks SQLite database for previously unfound tracks
7. **API Query** - Fetches lyrics from lrclib.net (rate-limited)
8. **Atomic Write** - Saves synchronized lyrics as `.lrc` files
9. **Session Update** - Updates session state for resume capability

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    TUI (Ratatui)                        │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐   │
│  │ Header  │  │ Progress │  │   Logs   │  │ Footer  │   │
│  └─────────┘  └──────────┘  └──────────┘  └─────────┘   │
└───────────────────┬─────────────────────────────────────┘
                    │ mpsc channels (bidirectional)
                    ▼
┌─────────────────────────────────────────────────────────┐
│                   Worker (Tokio)                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐   │
│  │ Scanner  │→ │  Cache   │→ │ LRCLIB   │→ │  .lrc  │   │
│  │ (walkdir)│  │ (SQLite) │  │   API    │  │ Writer │   │
│  └──────────┘  └──────────┘  └──────────┘  └────────┘   │
│       ↓                                          ↓      │
│  ┌──────────────────────────────────────────────────┐   │
│  │         Session Persistence (Atomic)             │   │
│  │  - Pause/Resume state                            │   │
│  │  - Progress counts                               │   │
│  │  - Log history (500 entries)                     │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## 📁 Data Storage

All persistent data follows XDG Base Directory specification:

### File Locations

| File | Path | Purpose |
|------|------|---------|
| **Cache Database** | `~/.local/share/getlrc/negative_cache.db` | Stores tracks not found on lrclib.net |
| **Session File** | `~/.local/share/getlrc/session.json` | Saves progress when paused |
| **Debug Logs** | `~/.local/share/getlrc/logs/getlrc.log` | All operations and errors |

### Session File Structure

```json
{
  "root_path": "/home/user/Music",
  "pending_files": ["file3.mp3", "file4.flac"],
  "downloaded_count": 42,
  "cached_count": 15,
  "existing_count": 8,
  "failed_count": 3,
  "log_history": [
    { "filename": "song1.mp3", "status": "Downloaded" },
    { "filename": "song2.flac", "status": "Cached" }
  ]
}
```

**Features**:
- ⚛️ Atomic saves (temp file + rename)
- 🔍 Integrity checks (validates on load)
- 📝 Log capping (max 500 entries)
- 🧹 Auto-cleanup (deleted on completion)

### Logging

All debug output is written to the log file to keep the TUI clean:

**Log Contents**:
- Full file paths
- API request URLs
- Metadata extraction details
- Detailed error messages
- Timing information
- Session operations

**View Logs**:
```bash
# Real-time monitoring
tail -f ~/.local/share/getlrc/logs/getlrc.log

# Search for errors
grep ERROR ~/.local/share/getlrc/logs/getlrc.log

# View session operations
grep "Session" ~/.local/share/getlrc/logs/getlrc.log
```

## 🛠️ Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Lint (zero warnings required)
cargo clippy -- -D warnings

# Format code
cargo fmt

# Install locally for testing
cargo install --path . --force
```

### Project Structure

```
getlrc/
├── src/
│   ├── main.rs           # Entry point, CLI handling
│   ├── lib.rs            # Module exports
│   ├── api.rs            # lrclib.net API client
│   ├── cache.rs          # SQLite negative cache
│   ├── env.rs            # Environment verification
│   ├── messages.rs       # Worker ↔ TUI messages
│   ├── paths.rs          # XDG path utilities
│   ├── scanner/          # Directory scanning, metadata
│   ├── session.rs        # Session persistence
│   ├── tui/              # Terminal UI (Ratatui)
│   └── worker.rs         # Async processing logic
├── Cargo.toml            # Dependencies and metadata
└── README.md             # This file
```

### Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal control |
| `lofty` | Audio metadata extraction |
| `rusqlite` | SQLite database |
| `reqwest` | HTTP client (rustls) |
| `walkdir` | Directory traversal |
| `tracing` | Structured logging |
| `serde` | Serialization |
| `anyhow` | Error handling |

## 🗺️ Roadmap

### ✅ Completed Features

- [x] Basic lyrics fetching and saving
- [x] XDG Base Directory compliance
- [x] SQLite negative cache
- [x] Interactive TUI with real-time updates
- [x] Pause/Resume controls
- [x] Atomic session persistence
- [x] Log history restoration
- [x] Progress bar 100% completion
- [x] Environment verification
- [x] Integrity checks for stale sessions
- [x] Comprehensive logging

### 🚧 Next Priority: Multi-threaded Scanning

- [ ] Parallel directory walking with `rayon` or `jwalk`
- [ ] Concurrent API worker pool with rate limiting
- [ ] Async file I/O for metadata extraction
- [ ] Progress estimation and ETA

### 🔮 Future Enhancements

- [ ] Exponential backoff for API retries
- [ ] Configurable API rate limits
- [ ] Multiple API source support
- [ ] Lyrics quality scoring
- [ ] Batch processing modes
- [ ] Configuration file support

## 🐛 Troubleshooting

### Common Issues

**Issue**: Binary not found after installation
```bash
# Check if ~/.cargo/bin is in PATH
echo $PATH | grep cargo

# Add to PATH if missing (see Installation section)
```

**Issue**: Permission denied errors
```bash
# Check data directory permissions
ls -la ~/.local/share/getlrc/

# Fix permissions if needed
chmod 755 ~/.local/share/getlrc/
chmod 644 ~/.local/share/getlrc/*.db
```

**Issue**: Stale session detected
```bash
# This happens if files were deleted while app was closed
# The app will automatically start a fresh scan
# To force a fresh scan, delete the session file:
rm ~/.local/share/getlrc/session.json
```

**Issue**: Progress bar stuck before 100%
```bash
# This was fixed in v0.1.0
# Update to the latest version:
cargo install --path . --force
```

### Debug Mode

For detailed debugging, check the log file:
```bash
# View full log
cat ~/.local/share/getlrc/logs/getlrc.log

# Monitor in real-time
tail -f ~/.local/share/getlrc/logs/getlrc.log

# Search for specific errors
grep -i "error\|warn" ~/.local/share/getlrc/logs/getlrc.log
```

## 📄 License

MIT

## 🙏 Credits

- Lyrics provided by [lrclib.net](https://lrclib.net)
- Built with Rust 🦀
- UI powered by [Ratatui](https://github.com/ratatui-org/ratatui)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

### Development Guidelines

- Follow Rust idioms and best practices
- Maintain zero `cargo clippy` warnings
- Add tests for new features
- Update documentation
- Preserve surgical precision (minimal diff noise)

---

**Current Version**: 0.1.0  
**Status**: Production Ready  
**Next Milestone**: Multi-threaded Scanning

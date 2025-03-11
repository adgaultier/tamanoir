

## Keylogger (ebpf kprobe program)
- attached to `input_handle_event`
- keystrokes codes are stored inside a ringbuffer (read by egress program)

## Egress (ebpf tc program)
- when a udp packet with destination ip `HIJACK-IP` is  to be sent, ebpf program captures it 
- a payload is built, with target architecture stored as first byte 
- a fixed-length chunk of stored keystrokes is fetched from ringbuffer and added to payload
- it is injected between layer3 (IP) and layer4 (UDP) 
- destination ip is replaced by Tamanoir-C2 server IP
- packet is then sent

## Ingress (ebpf tc program)
- when a udp packet from Tamanoir-C2 server IP is recieved, ebpf program captures it
- source ip is replaced by `HIJACK-IP`
- end of dns response is parsed (additional data): if it contains  a chunk shellcode payload, it is extracted and stored in a dedicated ringbuffer (read by executor).

## Executor (user space)
- listen to ringbuffer events provided by ingress program
- store received bytes in memory. It will be the next shellcode payload to execute.
- when end-of-transmission byte is detected, execute shellcode
- if a reset byte is detected, curent shellcode bytes are flushed

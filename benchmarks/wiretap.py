"""Relay a Unix socket and count the bytes the server sends back.

    python3 -I wiretap.py LISTEN TARGET COUNTER

Every connection to LISTEN is forwarded to TARGET. COUNTER is an 8-byte file
holding the total bytes TARGET has sent, as a little-endian u64, kept in an
mmap so the harness can read it at any moment without asking the relay.
`bench.py --wire` puts this between Ship's client and server. It runs as its
own process, so the harness's reader never waits on it, but every byte takes
one more hop, so keep `--wire` runs out of latency comparisons.
"""

import mmap
import os
import socket
import struct
import sys
import threading

listen, target, counter = sys.argv[1:]
with open(counter, "wb") as f:
    f.write(bytes(8))
fd = os.open(counter, os.O_RDWR)
total = mmap.mmap(fd, 8)
lock = threading.Lock()
received = 0


def pipe(src, dst, count):
    global received
    try:
        while data := src.recv(1 << 18):
            if count:
                with lock:
                    received += len(data)
                    total[:] = struct.pack("<Q", received)
            dst.sendall(data)
    except OSError:
        pass
    for s in (src, dst):
        try:
            s.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass


server = socket.socket(socket.AF_UNIX)
server.bind(listen)
server.listen()
while True:
    client, _ = server.accept()
    upstream = socket.socket(socket.AF_UNIX)
    upstream.connect(target)
    threading.Thread(target=pipe, args=(client, upstream, False), daemon=True).start()
    threading.Thread(target=pipe, args=(upstream, client, True), daemon=True).start()

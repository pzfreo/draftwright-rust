#!/usr/bin/env python3
"""Exercise the local HTTP adapter's byte limits, deadlines and recovery."""

import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading
import time


HEAD_LIMIT = 16 * 1024
READ_DEADLINE = 10
REQUEST_LINE = b"POST /mcp HTTP/1.1\r\n"
PING = b'{"jsonrpc":"2.0","id":1,"method":"ping"}'
METRICS = {}


def request(body=PING):
    return (
        REQUEST_LINE
        + b"Content-Type: application/json\r\n"
        + b"Accept: application/json, text/event-stream\r\n"
        + f"Content-Length: {len(body)}\r\n\r\n".encode()
        + body
    )


def status(stream, *, drain=False):
    line = bytearray()
    try:
        while not line.endswith(b"\n"):
            byte = stream.recv(1)
            if not byte:
                return "closed"
            line.extend(byte)
        result = int(line.split()[1])
        if drain:
            while stream.recv(4096):
                pass
        return result
    except TimeoutError:
        return "timeout"
    except (OSError, ValueError, IndexError):
        return "closed"


class Server:
    def __init__(self, binary):
        self.cache = tempfile.TemporaryDirectory(prefix="draftwright-http-")
        self.child = subprocess.Popen(
            [str(binary), "mcp-http", "127.0.0.1:0"],
            env={**os.environ, "DRAFTWRIGHT_CACHE": self.cache.name},
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        line = self.child.stderr.readline().decode().strip()
        endpoint = line.removeprefix("draftwright MCP listening on http://")
        host, port = endpoint.removesuffix("/mcp").rsplit(":", 1)
        self.address = (host, int(port))

    def connect(self, timeout=1):
        return socket.create_connection(self.address, timeout=timeout)

    def close(self):
        self.child.kill()
        self.child.wait()
        self.child.stderr.close()
        self.cache.cleanup()


def refused(server, name, wire, trickle=False):
    stop = threading.Event()
    with server.connect() as hostile, server.connect() as following:
        hostile.sendall(wire)
        following.sendall(request())

        def drip():
            while not stop.wait(0.1):
                try:
                    hostile.sendall(b"x")
                except OSError:
                    break

        writer = threading.Thread(target=drip) if trickle else None
        if writer:
            writer.start()
        hostile.settimeout(READ_DEADLINE + 2 if trickle else 0.75)
        start = time.monotonic()
        # A trickling sender may race the refusal with another write after close.
        actual = status(hostile, drain=not trickle)
        elapsed = time.monotonic() - start
        stop.set()
        if writer:
            writer.join()
        # A deadline rejection must free the server while the sender stays connected.
        if actual == "timeout":
            hostile.shutdown(socket.SHUT_RDWR)
        recovery = status(following, drain=True)
    expected = 408 if trickle else 431
    METRICS[name] = {
        "status": actual,
        "expected": expected,
        "seconds": round(elapsed, 3),
        "following_status": recovery,
    }
    assert actual == expected, f"{name}: {actual}, expected {expected}"
    assert recovery == 200, f"{name}: following ping returned {recovery}"


def controls(server, binary):
    # Both CRLFs, all UTF-8 bytes and the request line count toward the same cap.
    for total, expected in [(HEAD_LIMIT, 200), (HEAD_LIMIT + 1, 431)]:
        base = request().split(b"\r\n\r\n", 1)[0] + b"\r\nX-Pad: "
        padding = b"\xc3\xa9" * ((total - len(base) - 4) // 2)
        padding += b"x" * (total - len(base) - len(padding) - 4)
        wire = base + padding + b"\r\n\r\n" + PING
        with server.connect() as stream:
            # Split the final CRLF across writes, leaving body bytes buffered normally.
            stream.sendall(wire[: total - 1])
            stream.sendall(wire[total - 1 :])
            actual = status(stream, drain=True)
            assert actual == expected, f"head size {total}: {actual}, expected {expected}"
    for extra, expected in [
        (b"Origin: https://malicious.example\r\n", 403),
        (f"Origin: http://127.0.0.1:{server.address[1]}\r\n".encode(), 200),
        (b"Transfer-Encoding: chunked\r\n", 400),
        (b"Content-Length: 8388609\r\n", 413),
    ]:
        with server.connect() as stream:
            stream.sendall(request().replace(b"\r\n\r\n", b"\r\n" + extra + b"\r\n"))
            assert status(stream, drain=True) == expected, f"transport guard {extra!r}"
    with server.connect() as stream:
        stream.sendall(request(PING + b" " * (8 * 1024 * 1024 - len(PING))))
        assert status(stream, drain=True) == 200, "body at exactly 8 MiB"
    with server.connect() as stream:
        stream.sendall(request())
        assert status(stream) == 200
        body = bytearray()
        while data := stream.recv(4096):
            body.extend(data)
        assert json.loads(body.split(b"\r\n\r\n", 1)[1])["result"] == {}
    result = subprocess.run(
        [str(binary), "mcp-http", "0.0.0.0:0"], capture_output=True, timeout=2
    )
    assert result.returncode != 0 and b"only loopback" in result.stderr
    METRICS["preserved_transport_rules"] = {"status": "pass"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    aggregate = REQUEST_LINE + b"X-Small: x\r\n" * 1000 + b"X-Last: "
    cases = [
        ("oversized_request_line", b"POST /" + b"x" * HEAD_LIMIT + b" HTTP/1.1\r\n", False),
        ("oversized_header", REQUEST_LINE + b"X-Large: " + b"x" * HEAD_LIMIT, False),
        ("aggregate_overflow", aggregate + b"x" * (HEAD_LIMIT + 1 - len(aggregate)), False),
        ("newline_free", b"x" * (HEAD_LIMIT + 1), False),
        ("slow_head", REQUEST_LINE + b"X-Slow: ", True),
        ("slow_body", request(b"x" * 1024).split(b"\r\n\r\n", 1)[0] + b"\r\n\r\n", True),
    ]
    failures = 0
    for name, wire, trickle in cases:
        server = Server(binary)
        try:
            refused(server, name, wire, trickle)
            print(f"PASS {name}")
        except AssertionError as error:
            failures += 1
            print(f"FAIL {error}")
        finally:
            server.close()
    server = Server(binary)
    try:
        controls(server, binary)
        print("PASS preserved_transport_rules")
    except AssertionError as error:
        failures += 1
        print(f"FAIL {error}")
    finally:
        server.close()
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(METRICS, indent=2) + "\n")
    print(f"{7 - failures}/7 checks passed")
    return bool(failures)


if __name__ == "__main__":
    raise SystemExit(main())

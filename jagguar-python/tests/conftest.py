import subprocess
import time
from pathlib import Path
import pytest
import jagguar

ROOT = Path(__file__).resolve().parents[2]
SERVER_BIN = ROOT / "target" / "debug" / "server"
SERVER_ARGS = {
    "port": 8080,
    "security_parameter": 40,
    "corruption_threshold": 5,
    "reconstruction_threshold": 5,
    "committee_size": 9,
}

def _build_server() -> None:
    if SERVER_BIN.exists():
        return
    subprocess.check_call(
        ["cargo", "build", "--quiet", "--bin", "server"],
        cwd=ROOT,
    )

def _wait_until_ready() -> None:
    for _ in range(50):
        try:
            jagguar.Client().request_state()
            return
        except RuntimeError:
            time.sleep(0.05)
    raise RuntimeError("server did not become ready on 127.0.0.1:8080")

@pytest.fixture(scope="session")
def server():
    _build_server()
    proc = subprocess.Popen(
        [
            str(SERVER_BIN),
            str(SERVER_ARGS["port"]),
            str(SERVER_ARGS["security_parameter"]),
            str(SERVER_ARGS["corruption_threshold"]),
            str(SERVER_ARGS["reconstruction_threshold"]),
            str(SERVER_ARGS["committee_size"]),
        ],
        cwd=ROOT,
    )
    try:
        _wait_until_ready()
        yield proc
    finally:
        proc.kill()
        proc.wait()

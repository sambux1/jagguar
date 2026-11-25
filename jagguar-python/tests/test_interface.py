import pytest
import jagguar

def test_imports():
    assert hasattr(jagguar, "Client")
    assert hasattr(jagguar, "run_client")

def test_basic():
    client = jagguar.Client()
    assert isinstance(client, jagguar.Client)
    assert client.set_input([1, 2, 3]) is None

def test_no_server():
    client = jagguar.Client()
    with pytest.raises(RuntimeError):
        client.request_state()

def test_request_state(server):
    client = jagguar.Client()
    assert client.request_state() is None

def test_encrypt_input(server):
    client = jagguar.Client()
    assert client.request_state() is None
    client.set_input([1, 2, 3])
    assert client.encrypt_input() is None

def test_send_input(server):
    client = jagguar.Client()
    assert client.request_state() is None
    client.set_input([1, 2, 3])
    client.encrypt_input()
    assert client.send_input() is None

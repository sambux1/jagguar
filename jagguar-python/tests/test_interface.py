import jagguar

def test_imports():
    assert hasattr(jagguar, "Client")
    assert hasattr(jagguar, "get_state")
    assert hasattr(jagguar, "run_client")

def test_basic():
    client = jagguar.Client()
    client.set_input([1, 2, 3])

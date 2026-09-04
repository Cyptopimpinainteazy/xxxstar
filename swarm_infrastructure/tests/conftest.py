"""Pytest fixtures for the swarm test suite.

The original tests expect the ``loop`` and ``aiohttp_server`` fixtures
provided by the ``pytest-asyncio`` plugin.  In recent versions of the
plugin the ``loop`` fixture was removed in favour of the built‑in
``asyncio_loop`` fixture.  The tests also use ``aiohttp_server`` which
is provided by the ``pytest-aiohttp`` plugin.  Since that plugin is not
available in this environment we provide minimal replacements that
delegate to the standard fixtures.

These fixtures are intentionally lightweight and only provide the
attributes required by the tests.
"""

import pytest
from aiohttp import web


@pytest.fixture
def loop(event_loop):
    """Compatibility shim for the removed ``loop`` fixture.

    The tests use ``loop`` to access the event loop.  ``pytest-asyncio``
    exposes ``asyncio_loop`` (or ``event_loop`` in older versions).  We
    simply return the loop instance.
    """
    return event_loop


@pytest.fixture
async def aiohttp_server(aiohttp_client, event_loop):
    """Provide a minimal ``aiohttp_server`` fixture.

    The original fixture returns a server object with a ``make_url``
    method.  ``aiohttp_client`` already creates a test client; we wrap
    it to expose the required API.
    """
    class _Server:
        def __init__(self, client):
            self._client = client

        def make_url(self, path=""):
            return self._client.make_url(path)

    async def _create(app):
        client = await aiohttp_client(app)
        return _Server(client)

    return _create

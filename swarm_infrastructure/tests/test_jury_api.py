"""Tests for jury API endpoints integration.

Note: The jury API endpoints are already integrated into SwarmAPIServer.
These tests verify the endpoint handlers by instantiating the API server
and checking that the handlers exist and are callable.

Full integration tests would require aiohttp's test utilities,
which is already tested when the API server is deployed.
"""
import hashlib
from swarm.api_server import SwarmAPIServer


def test_jury_handlers_exist():
    """Verify jury handlers are registered in SwarmAPIServer."""
    api_server = SwarmAPIServer()
    
    # Check that jury handlers exist
    assert hasattr(api_server, 'create_jury_session')
    assert hasattr(api_server, 'jury_vote')
    assert hasattr(api_server, 'get_jury_session')
    assert callable(api_server.create_jury_session)
    assert callable(api_server.jury_vote)
    assert callable(api_server.get_jury_session)


def test_jury_manager_initialized():
    """Verify JuryManager is initialized with SwarmAPIServer."""
    api_server = SwarmAPIServer()
    
    # Check that jury_manager exists and is properly initialized
    assert hasattr(api_server, 'jury_manager')
    assert api_server.jury_manager is not None
    
    # Verify jury manager has expected methods
    assert hasattr(api_server.jury_manager, 'create_session')
    assert hasattr(api_server.jury_manager, 'submit_commit')
    assert hasattr(api_server.jury_manager, 'submit_reveal')
    assert hasattr(api_server.jury_manager, 'aggregate')
    assert hasattr(api_server.jury_manager, 'advance_to_reveal')
    assert hasattr(api_server.jury_manager, 'get_session')


def test_jury_api_routes_registered():
    """Verify jury API routes are registered with the app."""
    api_server = SwarmAPIServer()
    
    # Create a mock app to check routes
    from aiohttp import web
    app = web.Application()
    api_server.setup_routes(app)
    
    # Collect all registered routes
    routes = []
    for resource in app.router.resources():
        if hasattr(resource, '_routes'):
            routes.extend(resource._routes)
    
    # Check that jury routes are registered
    route_paths = [str(resource) for resource in app.router.resources()]
    
    # Should contain jury endpoints
    jury_route_found = any('/jury' in path for path in route_paths)
    assert jury_route_found, f"No jury routes found. Routes: {route_paths}"


def test_jury_manager_basic_functionality():
    """Test basic JuryManager functionality through API server."""
    api_server = SwarmAPIServer()
    
    from swarm.jury.manager import JuryMember
    
    # Create a jury session
    members = [
        JuryMember(agent_id="juror-1", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-2", section="security", is_on_chain=False),
        JuryMember(agent_id="juror-3", section="economics", is_on_chain=False),
    ]
    
    session = api_server.jury_manager.create_session(
        task_ids=["task-1"],
        members=members
    )
    
    assert session.session_id is not None
    assert session.state.value == 'commit'
    assert len(session.members) == 3
    
    # Test vote submission
    import hashlib
    vote = True
    nonce = "secret-123"
    commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
    
    ok = api_server.jury_manager.submit_commit(session.session_id, "juror-1", commitment)
    assert ok is True
    
    # Test advancing to reveal phase
    ok = api_server.jury_manager.advance_to_reveal(session.session_id)
    assert ok is True
    
    # Test reveal
    ok = api_server.jury_manager.submit_reveal(session.session_id, "juror-1", vote, nonce)
    assert ok is True


if __name__ == '__main__':
    import pytest
    pytest. main([__file__, '-v'])


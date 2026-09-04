"""Small error types and aiohttp middleware used by Swarm API

Provides:
- APIError: controlled API exception with status and details
- ExternalServiceError: 502 wrapper for downstream failures
- error_middleware: aiohttp middleware to convert exceptions to JSON responses
"""
from aiohttp import web
import logging
from typing import Any, Dict, Optional

logger = logging.getLogger(__name__)


class APIError(Exception):
    def __init__(self, message: str, status: int = 400, details: Optional[Dict[str, Any]] = None):
        super().__init__(message)
        self.message = message
        self.status = status
        self.details = details or {}


class ExternalServiceError(APIError):
    def __init__(self, message: str = "external_service_error", details: Optional[Dict[str, Any]] = None):
        super().__init__(message, status=502, details={'detail': details} if details else {})


@web.middleware
async def error_middleware(request: web.Request, handler):
    try:
        return await handler(request)
    except APIError as e:
        payload = {'error': e.message}
        if e.details:
            payload['details'] = e.details
        return web.json_response(payload, status=e.status)
    except web.HTTPException:
        # Let aiohttp's HTTPExceptions through
        raise
    except Exception as e:
        logger.exception('Unhandled API error')
        return web.json_response({'error': 'internal_server_error', 'detail': str(e)}, status=500)

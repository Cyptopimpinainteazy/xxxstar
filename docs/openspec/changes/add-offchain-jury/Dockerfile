# Multi-stage Dockerfile for X3 Chain Jury Service
# Supports both CPU and GPU (CUDA) builds
#
# Build arguments:
#   PYTHON_VERSION: Python version to use (default: 3.11)
#   WITH_GPU: Enable GPU/CUDA support (default: false)
#
# Usage:
#   CPU: docker build --build-arg PYTHON_VERSION=3.11 --build-arg WITH_GPU=false -t jury-service .
#   GPU: docker build --build-arg PYTHON_VERSION=3.11 --build-arg WITH_GPU=true -t jury-service:gpu .

ARG PYTHON_VERSION=3.11
ARG WITH_GPU=false

# ============================================================
# Base Stage: Python runtime
# ============================================================
FROM python:${PYTHON_VERSION}-slim as base

# Set working directory
WORKDIR /app

# Set environment variables
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    git \
    gcc \
    g++ \
    libssl-dev \
    libffi-dev \
    python3-dev \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Builder Stage: Compile dependencies
# ============================================================
FROM base as builder

WORKDIR /app

# Copy requirements
COPY swarm/requirements.txt .

# Install Python dependencies to a virtual environment
RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"
RUN pip install --upgrade pip setuptools wheel && \
    pip install -r requirements.txt

# Add additional dependencies for API server and jury module
RUN pip install \
    aiohttp-cors>=0.7.0 \
    aiofiles>=23.0 \
    python-dotenv>=1.0 \
    pydantic>=2.0 \
    pydantic-settings>=2.0 \
    sqlalchemy>=2.0 \
    alembic>=1.12 \
    psycopg2-binary>=2.9 \
    redis>=5.0 \
    httpx>=0.25 \
    prometheus-client>=0.18 \
    python-dateutil>=2.8

# ============================================================
# GPU Stage (optional): CUDA runtime
# ============================================================
FROM nvidia/cuda:12.1.1-runtime-ubuntu22.04 as gpu
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Final Stage: Runtime image
# ============================================================
FROM base as runtime

WORKDIR /app

# Copy virtual environment from builder
COPY --from=builder /opt/venv /opt/venv

# Create non-root user for security
RUN useradd -m -u 1000 -U -s /bin/bash jury && \
    mkdir -p /var/log/jury /var/cache/jury/sessions && \
    chown -R jury:jury /app /var/log/jury /var/cache/jury

# Set environment
ENV PATH="/opt/venv/bin:$PATH" \
    VIRTUAL_ENV="/opt/venv"

# Copy application code
COPY --chown=jury:jury swarm /app/swarm

# Switch to non-root user
USER jury

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 --start-period=20s \
    CMD curl -f http://localhost:8000/health || exit 1

# Default command
CMD ["python", "-m", "swarm.api_server"]

# ============================================================
# Labels
# ============================================================
LABEL description="X3 Chain Off-Chain Jury Service" \
      maintainer="x3-chain-dev" \
      version="1.0.0" \
      org.opencontainers.image.title="x3-jury-service" \
      org.opencontainers.image.description="Off-chain jury system for major task governance approval" \
      org.opencontainers.image.source="https://github.com/Cyptopimpinainteazy/x3-chain"

# Expose ports
EXPOSE 8000 8001

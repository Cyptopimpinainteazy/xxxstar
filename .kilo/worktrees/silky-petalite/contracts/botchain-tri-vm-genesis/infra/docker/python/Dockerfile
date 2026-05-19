FROM python:3.11-slim

LABEL maintainer="Botchain Team"
LABEL description="Python services for Botchain Tri-VM MVP"

# Set environment variables
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV PIP_NO_CACHE_DIR=1
ENV PIP_DISABLE_PIP_VERSION_CHECK=1

# Install system dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy requirements first for caching
COPY requirements.txt .

# Install Python dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY python/ ./python/
COPY compiler/ ./compiler/
COPY samples/ ./samples/

# Create necessary directories
RUN mkdir -p /app/keys /app/artifacts /app/data

# Create non-root user
RUN useradd -m -u 1000 botchain && \
    chown -R botchain:botchain /app

USER botchain

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Default command (can be overridden)
CMD ["python", "-m", "python.checker.checker"]

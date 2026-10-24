FROM python:3.10-slim

WORKDIR /app

COPY pyproject.toml README.md /app/
COPY src /app/src
COPY dashboard /app/dashboard
COPY kernels /app/kernels

RUN pip install --no-cache-dir -e .

CMD ["ccgv", "orchestrator"]

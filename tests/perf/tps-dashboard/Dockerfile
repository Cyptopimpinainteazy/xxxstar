FROM python:3.11-slim

WORKDIR /app

# Copy requirements and install
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy dashboard script
COPY dashboard.py .

# Expose Streamlit port
EXPOSE 8501

# Configure Streamlit
RUN mkdir -p ~/.streamlit && echo "\
[server]\n\
headless = true\n\
port = 8501\n\
[logger]\n\
level = info\n\
" > ~/.streamlit/config.toml

# Run Streamlit
CMD ["streamlit", "run", "dashboard.py"]

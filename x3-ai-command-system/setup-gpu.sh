#!/bin/bash
sudo cp /home/x3star/Desktop/xxxstar-main/x3-ai-command-system/ollama-override.conf /etc/systemd/system/ollama.service.d/override.conf
sudo systemctl daemon-reload
sudo systemctl restart ollama
echo "GPU optimization applied!"
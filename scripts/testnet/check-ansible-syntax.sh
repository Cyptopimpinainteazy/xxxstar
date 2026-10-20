#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export ANSIBLE_CONFIG="$ROOT/ansible/ansible.cfg"
for playbook in "$ROOT"/ansible/playbooks/*.yml; do
  ansible-playbook --syntax-check -i "$ROOT/ansible/inventory/hosts.seven.example.yml" "$playbook"
done

#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_URL="http://127.0.0.1:8787"

usage() {
  cat <<EOF
Usage: $0 <command> [task-id|output-file]
Commands:
  list                 List current swarm tasks from the API
  status <task-id>     Show a single task status
  approve <task-id>    Approve a manual task for worker processing
  reject <task-id>     Reject a manual task
  summary              Summarize completed and pending swarm tasks
  report [output-file] Generate a swarm task summary report

Examples:
  $0 list
  $0 approve x3-task-0001
  $0 summary
  $0 report reports/swarm_task_summary.md

Note:
  Use the `report` command after the swarm API is running to persist current task statuses into a markdown summary file.
EOF
  exit 1
}

if [ "$#" -lt 1 ]; then
  usage
fi

command="$1"
shift || true

task_arg="${1:-}"

if { [ "$command" = "status" ] || [ "$command" = "approve" ] || [ "$command" = "reject" ]; } && [ -z "$task_arg" ]; then
  echo "ERROR: task-id is required for '$command'"
  usage
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: curl is required to talk to the swarm API"
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required to parse swarm API responses"
  exit 1
fi

case "$command" in
  list)
    curl -fsS "$API_URL/tasks" | python3 -c 'import json,sys; tasks=json.load(sys.stdin); print("TASK_ID\tSTATUS\tAPPROVAL\tTITLE")
for t in tasks:
    print("{}\t{}\t{}\t{}".format(t.get("id", ""), t.get("status", ""), t.get("approval_required", ""), t.get("title", "")))'
    ;;
  status)
    curl -fsS "$API_URL/tasks/$task_arg" | python3 -c 'import json,sys; t=json.load(sys.stdin); print(json.dumps(t, indent=2))'
    ;;
  approve)
    curl -fsS -X POST "$API_URL/tasks/$task_arg/approve" | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin), indent=2))'
    ;;
  reject)
    curl -fsS -X POST "$API_URL/tasks/$task_arg/reject" | python3 -c 'import json,sys; print(json.dumps(json.load(sys.stdin), indent=2))'
    ;;
  summary)
    curl -fsS "$API_URL/tasks" | python3 -c 'import json,sys; tasks=json.load(sys.stdin); status_count={};
for t in tasks:
  status=t.get("status", "Unknown"); status_count[status]=status_count.get(status,0)+1
print("Total tasks: {}".format(len(tasks)))
for status in sorted(status_count.keys()):
    print("{}: {}".format(status, status_count[status]))
print("\nCompleted tasks:")
for t in tasks:
  if t.get("status") in ["Passed","Failed","Rejected"]:
    print("- {}: {}".format(t.get("id", ""), t.get("title", "")))'
    ;;
  report)
  output_file="${task_arg:-$ROOT_DIR/reports/swarm_task_summary.md}"
  mkdir -p "$(dirname "$output_file")"
    temp_file=$(mktemp)
    trap 'rm -f "$temp_file"' EXIT
    curl -fsS "$API_URL/tasks" > "$temp_file"
    OUTPUT_FILE="$output_file" TASK_FILE="$temp_file" python3 <<'PY'
import json, os
output = os.environ['OUTPUT_FILE']
with open(os.environ['TASK_FILE']) as f:
    tasks = json.load(f)
status_count = {}
for t in tasks:
  status = t.get('status', 'Unknown')
  status_count[status] = status_count.get(status, 0) + 1
with open(output, 'w') as f:
    f.write('# X3 Swarm Task Summary\n\n')
    f.write(f'Total tasks: {len(tasks)}\n')
    for status in sorted(status_count.keys()):
        f.write(f'- {status}: {status_count[status]}\n')
    f.write('\n## Completed tasks\n')
    for t in tasks:
      if t.get('status') in ['Passed', 'Failed', 'Rejected']:
        f.write(f'- {t.get("id", "")}: {t.get("title", "")} ({t.get("status", "Unknown")})\n')
    f.write('\n## Pending manual approvals\n')
    for t in tasks:
      if t.get('status') == 'Pending' and t.get('approval_required') == 'manual':
        f.write(f'- {t.get("id", "")}: {t.get("title", "")}\n')
print(output)
PY
    ;;
  *)
    echo "ERROR: unknown command '$command'"
    usage
    ;;
esac

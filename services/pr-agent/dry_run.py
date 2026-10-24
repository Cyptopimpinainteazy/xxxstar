"""Run the pr-agent CLI with publishing off and print what it would post.

With `config.publish_output=false` the review, describe and improve tools
store their markdown in `get_settings().data["artifact"]` (`ask` uses
`data["answer"]`) and return; the CLI never prints it. `improve` with nothing
to suggest stores an empty artifact, which is a result, not a failure. That
settings object lives in a per-request context that is gone by the time
`cli.run()` returns, so the artifact is read inside the request, by wrapping
`PRAgent.handle_request`.

Arguments are passed to the pr-agent CLI unchanged. Written against pr-agent
0.48.0; if a release moves the artifact, this exits 3 rather than print
nothing.
"""

import sys

from pr_agent import cli
from pr_agent.agent.pr_agent import PRAgent
from pr_agent.config_loader import get_settings

_handle_request = PRAgent.handle_request
_printed = False


async def _handle_and_print(self, *args, **kwargs):
    global _printed
    result = await _handle_request(self, *args, **kwargs)
    data = getattr(get_settings(), "data", None)
    if not hasattr(data, "get"):
        return result
    for key in ("artifact", "answer"):
        if key in data:
            print("\n===== pr-agent dry run: not published =====\n")
            print(data.get(key) or "(the tool ran; nothing to publish)")
            _printed = True
            break
    return result


PRAgent.handle_request = _handle_and_print

if __name__ == "__main__":
    status = cli.run()
    if status:
        sys.exit(status)
    if not _printed:
        print("error: dry run produced no output to print (see the log)",
              file=sys.stderr)
        sys.exit(3)

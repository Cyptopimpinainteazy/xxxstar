"""
Chat log ingestion — parses, normalizes, and indexes change requests.
Supports multiple chat formats: raw text, markdown, structured JSON, VSCode buffers.
"""
import hashlib
import re
from pathlib import Path
from typing import List, Optional

from md_supervisor.schema import ChangeRequest, ChangeIntent, ChatSource, FileTarget


def parse_chat_file(path: Path) -> List[ChangeRequest]:
    """Parse a chat log file and extract structured change requests."""
    text = path.read_text(encoding="utf-8", errors="replace")
    return parse_text(text, source=ChatSource.VSCODE_FILE)


def parse_text(text: str, source: ChatSource = ChatSource.MANUAL) -> List[ChangeRequest]:
    """Parse raw text for code/diff blocks and structured instructions."""
    requests: List[ChangeRequest] = []
    blocks = _extract_code_blocks(text)
    for block in blocks:
        cr = _block_to_change_request(block, source)
        if cr:
            requests.append(cr)
    return requests


def _extract_code_blocks(text: str) -> List[dict]:
    """Extract fenced code blocks with language annotations."""
    blocks = []
    pattern = r"```(\w+)?\n(.*?)```"
    for match in re.finditer(pattern, text, re.DOTALL):
        lang = match.group(1) or "text"
        code = match.group(2).strip()
        if code:
            blocks.append({"language": lang, "code": code})
    return blocks


def _block_to_change_request(block: dict, source: ChatSource) -> Optional[ChangeRequest]:
    """Convert a code block to a ChangeRequest with file target."""
    code = block["code"]
    lang = block["language"]
    content_hash = hashlib.sha256(code.encode()).hexdigest()

    # Infer file path from block annotations (e.g., #file: path/to/file.rs)
    path_hint = _infer_file_path(code, lang)

    ft = FileTarget(
        path=path_hint or f"unknown.{lang}",
        language=lang,
        original_hash="",
        proposed_content=code,
    )

    return ChangeRequest(
        source=source,
        files=[ft],
        intent=_classify_intent(lang, code),
        content_hash=content_hash,
        semantic_hash=content_hash,  # placeholder - real semantic hashing via AST
    )


def _infer_file_path(code: str, lang: str) -> Optional[str]:
    """Look for #file: markers in code blocks to determine target path."""
    match = re.search(r"#file:\s*(\S+)", code)
    if match:
        return match.group(1)
    # Also check for file path comments
    match = re.search(r"//\s*file:\s*(\S+)", code)
    if match:
        return match.group(1)
    match = re.search(r"<!--\s*file:\s*(\S+)\s*-->", code)
    if match:
        return match.group(1)
    return None


def _classify_intent(lang: str, code: str) -> ChangeIntent:
    """Classify whether a change is code, docs, config, or test."""
    # Test files
    if "test" in lang.lower() or "spec" in lang.lower():
        return ChangeIntent.TEST
    # Doc files
    if lang in ("markdown", "md", "rst", "txt"):
        return ChangeIntent.DOC
    # Config files
    if lang in ("json", "yaml", "toml", "ini", "cfg", "env"):
        return ChangeIntent.CONFIG
    return ChangeIntent.CODE


def compute_semantic_hash(code: str) -> str:
    """Compute a semantic hash that's invariant under whitespace and comment changes.
    Placeholder: uses normalized content until AST-based hashing is implemented.
    """
    normalized = re.sub(r"\s+", " ", code).strip()
    normalized = re.sub(r"//.*?\n|#.*?\n|/\*.*?\*/", "", normalized)
    return hashlib.sha256(normalized.encode()).hexdigest()
#!/usr/bin/env python3
"""
Simple block number visualization - one digit per block.
Special handling for milestones (1M, 10M, 100M blocks).

Block #5787 displays as 4 individual digit boxes.
Block #1000000 displays epic milestone art.

Usage: python3 block_display.py <BLOCK_NUMBER>
"""

import sys

# Neon rainbow colors
COLORS = [
    "\033[93m",   # Bright Yellow
    "\033[33m",   # Dark Yellow/Orange
    "\033[92m",   # Bright Green
    "\033[96m",   # Bright Cyan
    "\033[94m",   # Bright Blue
    "\033[35m",   # Magenta
    "\033[91m",   # Bright Red
    "\033[92m",   # Bright Green (cycle)
]
RESET = "\033[0m"
BOLD = "\033[1m"
GOLD = "\033[38;5;220m"
DIAMOND = "\033[38;5;51m"
FLAME = "\033[38;5;208m"

def get_color(index):
    """Get color code for index."""
    return COLORS[index % len(COLORS)]

def is_milestone(block_num: int) -> bool:
    """Check if block number is a notable milestone."""
    if block_num <= 0:
        return False
    # 1M milestones (1M, 10M, 100M, etc.)
    if block_num % 1000000 == 0:
        return True
    # 100k milestones (100k, 200k, etc.)
    if block_num % 100000 == 0:
        return True
    # 1k milestones (1k, 2k, 3k, etc.)
    return block_num % 1000 == 0

def get_milestone_level(block_num: int) -> str:
    """Determine milestone level: tiny (1k), big (100k), or explosive (1M)."""
    if block_num % 1000000 == 0:
        return "explosive"
    elif block_num % 100000 == 0:
        return "big"
    elif block_num % 1000 == 0:
        return "tiny"
    return "normal"

def display_tiny_milestone(block_num: int) -> None:
    """Display small celebration for 1k, 2k, 3k milestones."""
    k_val = block_num // 1000
    print(f"\n{BOLD}\033[38;5;118m")  # Lime green
    print(f"  ✦ {k_val}k blocks finalized ✦")
    print(RESET)

def display_big_milestone(block_num: int) -> None:
    """Display large celebration for 100k, 200k, 300k milestones."""
    block_num // 1000
    hundred_k = block_num // 100000
    formatted_blocks = f"{block_num:,}"

    print(f"\n{BOLD}\033[38;5;226m")  # Bright yellow
    print("""
  ╔════════════════════════════════════╗
  ║                                    ║
  ║   ◆  MAJOR MILESTONE  ◆           ║""")
    print("  ║                                    ║")
    print(f"  ║   {formatted_blocks} Blocks Finalized    ║")
    print(f"  ║   ({hundred_k} × 100,000 Threshold)    ║")
    print("""  ║                                    ║
  ║   Consistency proven. Trust earned.║
  ║                                    ║
  ╚════════════════════════════════════╝
    """)
    print(RESET)

def display_milestone(block_num: int) -> None:
    """Display milestone based on level."""
    level = get_milestone_level(block_num)

    if level == "tiny":
        display_tiny_milestone(block_num)
    elif level == "big":
        display_big_milestone(block_num)
    elif level == "explosive":
        display_explosive_milestone(block_num)

def display_explosive_milestone(block_num: int) -> None:
    """Display epic explosive celebration for 1M, 10M, 100M milestones."""
    if block_num == 1000000:
        print(f"\n{BOLD}{GOLD}")
        print("""
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║              🎉  MILESTONE REACHED  🎉                       ║
    ║                                                               ║
    ║              ONE MILLION BLOCKS FINALIZED                    ║
    ║                                                               ║
    ║                    Block #1,000,000                          ║
    ║                                                               ║
    ║              ★ ★ ★ ★ ★ ★ ★ ★ ★ ★                          ║
    ║                                                               ║
    ║    The x3-chain consensus has achieved                        ║
    ║    unprecedented distributed agreement.                       ║
    ║                                                               ║
    ║    Genesis Block → 1 Million Blocks                          ║
    ║    A journey through finality & faith.                       ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
        """)
        print(RESET)
    elif block_num == 10000000:
        print(f"\n{BOLD}{DIAMOND}")
        print("""
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║         ◆◆◆  LEGENDARY MILESTONE  ◆◆◆                       ║
    ║                                                               ║
    ║            TEN MILLION BLOCKS FINALIZED                      ║
    ║                                                               ║
    ║                  Block #10,000,000                           ║
    ║                                                               ║
    ║    The x3-chain stands eternal.                              ║
    ║    Ten million times proven true.                            ║
    ║                                                               ║
    ║    Throughput. Finality. Trust.                              ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
        """)
        print(RESET)
    elif block_num == 100000000:
        print(f"\n{BOLD}{FLAME}")
        print("""
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║         ★★★★★  ETERNAL CONSENSUS  ★★★★★                   ║
    ║                                                               ║
    ║          ONE HUNDRED MILLION BLOCKS FINALIZED                ║
    ║                                                               ║
    ║                 Block #100,000,000                           ║
    ║                                                               ║
    ║  The x3-chain burns brightly across eternity.                ║
    ║  One hundred million acts of distributed will.               ║
    ║                                                               ║
    ║  Beyond measure. Beyond question. Beyond doubt.              ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
        """)
        print(RESET)
    else:
        # Generic million milestone
        m_val = block_num // 1000000
        print(f"\n{BOLD}{GOLD}")
        print(f"    🎆  {m_val} MILLION BLOCKS FINALIZED  🎆")
        print(f"           Block #{block_num:,}")
        print(RESET)

def display_block_number(block_num: int) -> None:
    """Display block number as individual digit boxes."""
    # Check for milestones first
    if is_milestone(block_num):
        display_milestone(block_num)
        return

    # Get digits
    digits = str(block_num)

    # Header
    print(f"\n{BOLD}Block #{block_num}{RESET}\n")

    # Top line: ┌──┐ ┌──┐ ┌──┐ ...
    top_line = ""
    for i in range(len(digits)):
        color = get_color(i)
        top_line += f"{color}┌──┐{RESET} "
    print(top_line)

    # Middle line: │5 │ │7 │ │8 │ ...
    mid_line = ""
    for i, digit in enumerate(digits):
        color = get_color(i)
        mid_line += f"{color}│{digit} │{RESET} "
    print(mid_line)

    # Bottom line: └──┘ └──┘ └──┘ ...
    bot_line = ""
    for i in range(len(digits)):
        color = get_color(i)
        bot_line += f"{color}└──┘{RESET} "
    print(bot_line)
    print()

def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: python3 block_display.py <BLOCK_NUMBER>")
        print("Example: python3 block_display.py 5787")
        sys.exit(1)

    try:
        block_num = int(sys.argv[1])
        if block_num < 1:
            print("Error: Block number must be >= 1")
            sys.exit(1)
        display_block_number(block_num)
    except ValueError:
        print(f"Error: '{sys.argv[1]}' is not a valid integer")
        sys.exit(1)

if __name__ == "__main__":
    main()

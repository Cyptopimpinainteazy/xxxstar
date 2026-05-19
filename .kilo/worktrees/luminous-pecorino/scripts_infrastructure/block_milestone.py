#!/usr/bin/env python3
"""
Special milestone block displays with epic ASCII art.
Triggered for block numbers like: 1M, 10M, 100M, etc.
"""

import sys

# ANSI styles
RESET = "\033[0m"
BOLD = "\033[1m"
GOLD = "\033[38;5;220m"
DIAMOND = "\033[38;5;51m"
FLAME = "\033[38;5;208m"

def display_one_million() -> None:
    """Epic display for block 1,000,000 (1M milestone)."""

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

def display_ten_million() -> None:
    """Epic display for block 10,000,000 (10M milestone)."""
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

def display_hundred_million() -> None:
    """Epic display for block 100,000,000 (100M milestone)."""
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

def is_milestone(block_num: int) -> bool:
    """Check if block number is a notable milestone."""
    # Check for powers of 10: 1M, 10M, 100M, 1B, etc.
    str_num = str(block_num)
    if len(str_num) >= 7:  # 1M+ blocks
        # Check if it's 1 followed by zeros, or X followed by zeros
        if str_num[0] in ['1', '5'] and all(c == '0' for c in str_num[1:]):
            return True
    return False

def display_milestone(block_num: int) -> None:
    """Display appropriate milestone screen."""
    if block_num == 1000000:
        display_one_million()
    elif block_num == 10000000:
        display_ten_million()
    elif block_num == 100000000:
        display_hundred_million()
    else:
        # Generic milestone
        print(f"\n{BOLD}{GOLD}")
        print(f"✦ Milestone Block #{block_num:,} Finalized ✦")
        print(RESET)

def main() -> None:
    if len(sys.argv) < 2:
        display_one_million()
        return

    try:
        block_num = int(sys.argv[1])
        if is_milestone(block_num):
            display_milestone(block_num)
        else:
            print(f"Block #{block_num} is not a milestone")
    except ValueError:
        print(f"Error: Invalid block number '{sys.argv[1]}'")
        sys.exit(1)

if __name__ == "__main__":
    main()

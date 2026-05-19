#!/usr/bin/env python3
"""
Terminal Block Visualizer - Displays blocks with neon rainbow borders in the terminal.
Generates ASCII art blocks like the image: numbered 1-8 with dashed neon gradient borders.
"""

import sys


class Color:
    """ANSI color codes for neon effects."""
    RESET = "\033[0m"
    BRIGHT_YELLOW = "\033[93m"
    BRIGHT_GREEN = "\033[92m"
    BRIGHT_CYAN = "\033[96m"
    BRIGHT_BLUE = "\033[94m"
    BRIGHT_MAGENTA = "\033[95m"
    BRIGHT_RED = "\033[91m"
    BRIGHT_WHITE = "\033[97m"

    # Gradient sequence for rainbow effect
    GRADIENTS = [
        "\033[93m",   # Bright Yellow (block 1)
        "\033[33m",   # Orange (block 2)
        "\033[92m",   # Bright Green (block 3)
        "\033[96m",   # Bright Cyan (block 4)
        "\033[94m",   # Bright Blue (block 5)
        "\033[35m",   # Magenta (block 6)
        "\033[91m",   # Bright Red (block 7)
        "\033[92m",   # Bright Green (block 8)
    ]


class BlockVisualizer:
    """Renders blocks with neon rainbow borders."""

    def __init__(self, width: int = 15, height: int = 7) -> None:
        """
        Initialize the visualizer.

        Args:
            width: Width of each block
            height: Height of each block
        """
        self.width = width
        self.height = height
        self.border_char = "─"
        self.corner_char = "┌"
        self.corner_char_bottom_left = "└"
        self.corner_char_bottom_right = "┘"
        self.corner_char_top_right = "┐"
        self.vertical_char = "│"

    def render_single_block(self, block_num: int, color_code: str) -> list[str]:
        """
        Render a single block with neon border.

        Args:
            block_num: Block number (1-8)
            color_code: ANSI color code

        Returns:
            List of strings representing each line of the block
        """
        lines = []

        # Top border with dashes
        top_border = f"{color_code}┌{self.border_char * (self.width - 2)}┐{Color.RESET}"
        lines.append(top_border)

        # Empty lines with vertical borders
        for _ in range(self.height - 3):
            line = f"{color_code}│{' ' * (self.width - 2)}│{Color.RESET}"
            lines.append(line)

        # Middle line with number
        num_str = str(block_num)
        padding_left = (self.width - 4) // 2
        padding_right = self.width - 4 - padding_left
        center_line = f"{color_code}│{' ' * padding_left}{color_code}{num_str}{Color.RESET}{color_code}{' ' * padding_right}│{Color.RESET}"
        lines.append(center_line)

        # Bottom border with dashes
        bottom_border = f"{color_code}└{self.border_char * (self.width - 2)}┘{Color.RESET}"
        lines.append(bottom_border)

        return lines

    def render_blocks_horizontal(self, num_blocks: int = 8, show_dashed: bool = True) -> str:
        """
        Render blocks horizontally side by side.

        Args:
            num_blocks: Number of blocks to render (1-8)
            show_dashed: Use dashed borders for fancier effect

        Returns:
            String containing the entire rendered block pattern
        """
        if num_blocks > 8:
            num_blocks = 8
        if num_blocks < 1:
            num_blocks = 1

        # Create all blocks
        all_blocks = []
        for i in range(num_blocks):
            color = Color.GRADIENTS[i]
            block_lines = self.render_single_block(i + 1, color)
            all_blocks.append(block_lines)

        # Combine blocks horizontally
        output_lines = []
        for line_idx in range(len(all_blocks[0])):
            line_parts = []
            for block_idx in range(len(all_blocks)):
                line_parts.append(all_blocks[block_idx][line_idx])
            output_lines.append("".join(line_parts))

        return "\n".join(output_lines)

    def render_blocks_with_dashes(self, num_blocks: int = 8) -> str:
        """
        Render blocks with decorative dashed borders (like in the image).

        Args:
            num_blocks: Number of blocks to render

        Returns:
            String with dashed borders
        """
        if num_blocks > 8:
            num_blocks = 8
        if num_blocks < 1:
            num_blocks = 1

        output_lines = []

        # Top dashed border
        top_border = ""
        for i in range(num_blocks):
            color = Color.GRADIENTS[i]
            top_border += f"{color}┌{'─' * (self.width - 2)}┐{Color.RESET}"
        output_lines.append(top_border)

        # Content lines
        for line_idx in range(self.height - 2):
            line = ""
            for i in range(num_blocks):
                color = Color.GRADIENTS[i]
                if line_idx == self.height - 3:  # Center line with number
                    num_str = str(i + 1)
                    padding_left = (self.width - 4) // 2
                    padding_right = self.width - 4 - padding_left
                    line += f"{color}│{' ' * padding_left}{num_str}{' ' * padding_right}│{Color.RESET}"
                else:
                    line += f"{color}│{' ' * (self.width - 2)}│{Color.RESET}"
            output_lines.append(line)

        # Bottom dashed border
        bottom_border = ""
        for i in range(num_blocks):
            color = Color.GRADIENTS[i]
            bottom_border += f"{color}└{'─' * (self.width - 2)}┘{Color.RESET}"
        output_lines.append(bottom_border)

        return "\n".join(output_lines)


def display_blocks(num_blocks: int = 8, dashed: bool = True) -> None:
    """Display blocks in the terminal."""
    visualizer = BlockVisualizer()
    if dashed:
        output = visualizer.render_blocks_with_dashes(num_blocks)
    else:
        output = visualizer.render_blocks_horizontal(num_blocks)
    print(output)


if __name__ == "__main__":
    if len(sys.argv) > 1:
        try:
            num = int(sys.argv[1])
            display_blocks(num)
        except ValueError:
            display_blocks()
    else:
        display_blocks()

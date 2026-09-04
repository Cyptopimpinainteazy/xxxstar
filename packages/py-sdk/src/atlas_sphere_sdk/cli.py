"""
X3 Chain CLI - Command-line interface for the SDK.
"""

import typer
from rich.console import Console
from rich.table import Table
from typing import Optional

from x3_chain_sdk import AtlasClient
from x3_chain_sdk.types import AtlasError

app = typer.Typer(
    name="x3-chain",
    help="X3 Chain blockchain CLI",
    add_completion=False,
)
console = Console()


@app.command()
def info(
    url: str = typer.Option(
        "ws://localhost:9944",
        "--url", "-u",
        help="Node WebSocket URL",
    ),
):
    """Show chain information."""
    try:
        with AtlasClient(url) as client:
            chain = client.get_chain_info()
            
            table = Table(title="Chain Info")
            table.add_column("Property", style="cyan")
            table.add_column("Value", style="green")
            
            table.add_row("Chain Name", chain.chain_name)
            table.add_row("Chain ID", str(chain.chain_id))
            table.add_row("Token", f"{chain.token_symbol} ({chain.token_decimals} decimals)")
            table.add_row("SS58 Format", str(chain.ss58_format))
            table.add_row("Best Block", str(chain.best_number))
            table.add_row("Finalized", str(chain.finalized_number))
            table.add_row("Genesis Hash", chain.genesis_hash[:18] + "...")
            
            console.print(table)
    except AtlasError as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1)


@app.command()
def balance(
    account: str = typer.Argument(..., help="Account address (SS58)"),
    asset_id: int = typer.Option(0, "--asset", "-a", help="Asset ID"),
    url: str = typer.Option(
        "ws://localhost:9944",
        "--url", "-u",
        help="Node WebSocket URL",
    ),
):
    """Query account balance."""
    try:
        with AtlasClient(url) as client:
            bal = client.get_canonical_balance(account, asset_id)
            info = client.get_account_info(account)
            
            table = Table(title=f"Account: {account[:16]}...")
            table.add_column("Property", style="cyan")
            table.add_column("Value", style="green")
            
            table.add_row("Native Balance", f"{info.free_balance:,}")
            table.add_row("Reserved", f"{info.reserved_balance:,}")
            table.add_row("Canonical (Asset {})".format(asset_id), f"{bal:,}")
            table.add_row("Nonce", str(info.nonce))
            table.add_row("Authorized", "✓" if info.is_authorized else "✗")
            
            console.print(table)
    except AtlasError as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1)


@app.command()
def block(
    number: Optional[int] = typer.Argument(None, help="Block number (latest if omitted)"),
    url: str = typer.Option(
        "ws://localhost:9944",
        "--url", "-u",
        help="Node WebSocket URL",
    ),
):
    """Show block information."""
    try:
        with AtlasClient(url) as client:
            if number is not None:
                substrate = client._ensure_connected()
                block_hash = substrate.get_block_hash(number)
            else:
                block_hash = None
            
            header = client.get_block_header(block_hash)
            
            table = Table(title=f"Block #{header.number}")
            table.add_column("Property", style="cyan")
            table.add_column("Value", style="green")
            
            table.add_row("Hash", header.hash[:18] + "...")
            table.add_row("Parent", header.parent_hash[:18] + "...")
            table.add_row("State Root", header.state_root[:18] + "...")
            table.add_row("Extrinsics Root", header.extrinsics_root[:18] + "...")
            
            console.print(table)
    except AtlasError as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1)


@app.command()
def authorities(
    url: str = typer.Option(
        "ws://localhost:9944",
        "--url", "-u",
        help="Node WebSocket URL",
    ),
):
    """List current authorities."""
    try:
        with AtlasClient(url) as client:
            auths = client._call_rpc("atlasKernel_getAuthorities") or []
            
            if not auths:
                console.print("[yellow]No authorities found[/yellow]")
                return
            
            table = Table(title="Authorities")
            table.add_column("#", style="dim")
            table.add_column("Account", style="green")
            
            for i, auth in enumerate(auths, 1):
                table.add_row(str(i), auth)
            
            console.print(table)
    except AtlasError as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1)


@app.command()
def watch(
    url: str = typer.Option(
        "ws://localhost:9944",
        "--url", "-u",
        help="Node WebSocket URL",
    ),
    finalized: bool = typer.Option(False, "--finalized", "-f", help="Watch finalized blocks"),
):
    """Watch for new blocks."""
    import time
    
    console.print(f"[cyan]Connecting to {url}...[/cyan]")
    
    try:
        with AtlasClient(url) as client:
            mode = "finalized" if finalized else "new"
            console.print(f"[green]Watching {mode} blocks (Ctrl+C to stop)[/green]\n")
            
            def on_block(header):
                console.print(
                    f"Block [cyan]#{header.number}[/cyan] "
                    f"[dim]{header.hash[:16]}...[/dim]"
                )
            
            if finalized:
                client.subscribe_finalized_heads(on_block)
            else:
                client.subscribe_new_heads(on_block)
            
            # Keep running until Ctrl+C
            try:
                while True:
                    time.sleep(1)
            except KeyboardInterrupt:
                console.print("\n[yellow]Stopped watching[/yellow]")
    except AtlasError as e:
        console.print(f"[red]Error: {e}[/red]")
        raise typer.Exit(1)


def main():
    """CLI entry point."""
    app()


if __name__ == "__main__":
    main()

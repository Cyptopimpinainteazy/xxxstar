#!/usr/bin/env python3
"""
X3 Chain TPS Testing Orchestration Script

This script manages the complete TPS testing infrastructure:
- Builds and runs InfluxDB for metrics storage
- Runs the TPS tracker to collect blockchain metrics
- Starts the Streamlit dashboard for visualization

Adapted from Solana Project
"""

import argparse
import os
import subprocess
import sys
import time
import webbrowser
from pathlib import Path


class TPSTestOrchestrator:
    """Orchestrate TPS testing infrastructure"""

    def __init__(self, project_root: str | None = None):
        if project_root is None:
            project_root = Path(__file__).parent.parent.parent

        self.project_root = Path(project_root)
        self.docker_compose_file = self.project_root / "tests/perf/docker-compose.tps.yml"
        self.tps_tracker_crate = self.project_root / "crates/tps-tracker"

    def build_tps_tracker(self) -> bool:
        """Build the TPS tracker Rust crate"""
        print("📦 Building TPS Tracker...")
        try:
            result = subprocess.run(
                ["cargo", "build", "--release", "--manifest-path",
                 str(self.tps_tracker_crate / "Cargo.toml")],
                cwd=self.project_root,
                capture_output=True,
                text=True,
                timeout=300
            )

            if result.returncode != 0:
                print(f"❌ Build failed:\n{result.stderr}")
                return False

            print("✅ TPS Tracker built successfully")
            return True

        except subprocess.TimeoutExpired:
            print("❌ Build timeout")
            return False
        except Exception as e:
            print(f"❌ Build error: {e}")
            return False

    def start_services(self) -> bool:
        """Start Docker Compose services"""
        print("\n🐳 Starting Docker services...")

        # Set environment variables
        env = os.environ.copy()
        env["RPC_URL"] = os.getenv("RPC_URL", "http://127.0.0.1:9944")

        try:
            result = subprocess.run(
                ["docker-compose", "-f", str(self.docker_compose_file),
                 "up", "-d", "--build"],
                cwd=self.project_root,
                env=env,
                capture_output=True,
                text=True,
                timeout=120
            )

            if result.returncode != 0:
                print(f"❌ Docker compose failed:\n{result.stderr}")
                return False

            print("✅ Docker services started")
            return True

        except subprocess.TimeoutExpired:
            print("❌ Docker compose timeout")
            return False
        except Exception as e:
            print(f"❌ Docker error: {e}")
            return False

    def wait_for_dashboard(self, timeout: int = 60) -> bool:
        """Wait for Streamlit dashboard to be ready"""
        print("\n⏳ Waiting for dashboard...")

        start = time.time()
        while time.time() - start < timeout:
            try:
                result = subprocess.run(
                    ["curl", "-s", "-f", "http://localhost:8501"],
                    capture_output=True,
                    timeout=5
                )
                if result.returncode == 0:
                    print("✅ Dashboard is ready!")
                    return True
            except Exception:
                pass

            time.sleep(2)

        print("⚠️ Dashboard took too long to start (but may still be initializing)")
        return False

    def open_browser(self):
        """Open dashboard in default browser"""
        print("\n🌐 Opening dashboard in browser...")
        try:
            webbrowser.open("http://localhost:8501")
        except Exception as e:
            print(f"⚠️ Could not open browser: {e}")

    def show_logs(self):
        """Show service logs"""
        print("\n📋 Service Logs:\n")
        try:
            subprocess.run(
                ["docker-compose", "-f", str(self.docker_compose_file), "logs", "-f"],
                cwd=self.project_root
            )
        except KeyboardInterrupt:
            print("\n\nStopping logs...")

    def stop_services(self):
        """Stop Docker Compose services"""
        print("\n🛑 Stopping services...")
        try:
            subprocess.run(
                ["docker-compose", "-f", str(self.docker_compose_file), "down"],
                cwd=self.project_root,
                capture_output=True
            )
            print("✅ Services stopped")
        except Exception as e:
            print(f"❌ Error stopping services: {e}")

    def run(self, build: bool = True, open_browser: bool = True, show_logs: bool = True):
        """Run the complete TPS testing pipeline"""

        print("\n" + "="*60)
        print("  X3 Chain TPS Testing Infrastructure")
        print("="*60 + "\n")

        # Verify RPC connection
        rpc_url = os.getenv("RPC_URL", "http://127.0.0.1:9944")
        print(f"🔗 RPC URL: {rpc_url}")

        try:
            result = subprocess.run(
                ["curl", "-s", "-f", "-X", "POST", rpc_url,
                 "-H", "Content-Type: application/json",
                 "-d", '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}'],
                capture_output=True,
                timeout=5
            )
            if result.returncode == 0:
                print("✅ RPC connection verified\n")
            else:
                print(f"⚠️ RPC connection failed (may still work): {result.stderr}\n")
        except Exception:
            print("⚠️ Could not verify RPC connection\n")

        # Build TPS tracker
        if build and not self.build_tps_tracker():
            return False

        # Start services
        if not self.start_services():
            return False

        # Wait for dashboard
        time.sleep(5)  # Give services time to initialize
        self.wait_for_dashboard()

        # Open browser
        if open_browser:
            self.open_browser()

        # Show logs
        if show_logs:
            try:
                self.show_logs()
            except KeyboardInterrupt:
                print("\n\nShutting down...")
                self.stop_services()
                return False

        return True


def main():
    """Main entry point"""

    parser = argparse.ArgumentParser(
        description="X3 Chain TPS Testing Infrastructure"
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Skip building TPS tracker"
    )
    parser.add_argument(
        "--no-browser",
        action="store_true",
        help="Don't open dashboard in browser"
    )
    parser.add_argument(
        "--no-logs",
        action="store_true",
        help="Don't show service logs"
    )
    parser.add_argument(
        "--stop",
        action="store_true",
        help="Stop running services"
    )
    parser.add_argument(
        "--logs",
        action="store_true",
        help="Show service logs"
    )

    args = parser.parse_args()

    orchestrator = TPSTestOrchestrator()

    if args.stop:
        orchestrator.stop_services()
        return 0

    if args.logs:
        orchestrator.show_logs()
        return 0

    success = orchestrator.run(
        build=not args.no_build,
        open_browser=not args.no_browser,
        show_logs=not args.no_logs
    )

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())

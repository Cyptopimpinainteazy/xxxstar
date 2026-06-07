#!/usr/bin/env python3
"""
X3 Chain TPS Dashboard
Real-time visualization of blockchain transactions per second (TPS)

Adapted from Solana Project by Amil Shrivastava
For: X3 Chain Blockchain Platform
"""

import os
import time
from datetime import datetime

import pandas as pd
import plotly.graph_objects as go
import streamlit as st
from influxdb_client import InfluxDBClient
from influxdb_client.client.query_api import SYNCHRONOUS

# Page configuration
st.set_page_config(
    page_title="X3 Chain TPS Dashboard",
    page_icon="⚡",
    layout="wide",
    initial_sidebar_state="expanded"
)

# Custom CSS
st.markdown("""
    <style>
    .metric-card {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        color: white;
        padding: 20px;
        border-radius: 10px;
        text-align: center;
    }
    .metric-value {
        font-size: 36px;
        font-weight: bold;
        margin: 10px 0;
    }
    .metric-label {
        font-size: 14px;
        opacity: 0.9;
    }
    </style>
""", unsafe_allow_html=True)


class TPSDashboard:
    """X3 Chain TPS Dashboard"""

    def __init__(self):
        self.influx_url = os.getenv("INFLUX_URL", "http://localhost:8086")
        self.influx_token = os.getenv("INFLUX_TOKEN", "x3-chain-key")
        self.influx_db = os.getenv("INFLUX_DB", "x3_chain_tps")
        self.client = None
        self.connect()

    def connect(self):
        """Connect to InfluxDB"""
        try:
            self.client = InfluxDBClient(
                url=self.influx_url,
                token=self.influx_token,
                org="x3-chain"
            )
            self.client.buckets_api()  # Test connection
            st.session_state.db_connected = True
        except Exception as e:
            st.session_state.db_connected = False
            st.error(f"InfluxDB Connection Error: {e}")

    def fetch_data(self, time_range: str = "1h") -> pd.DataFrame:
        """Fetch TPS data from InfluxDB"""
        if not self.client or not st.session_state.get("db_connected"):
            return pd.DataFrame()

        try:
            query = f"""
            from(bucket: "{self.influx_db}")
            |> range(start: -{time_range})
            |> filter(fn: (r) => r["_measurement"] == "transaction_stats")
            |> sort(columns: ["_time"])
            """

            query_api = self.client.query_api(SYNCHRONOUS)
            tables = query_api.query(query)

            # Convert to DataFrame
            data_points = []
            for table in tables:
                for record in table.records:
                    data_points.append({
                        "time": record.get_time(),
                        "block_height": record.get("block_height", None),
                        "transaction_count": record.get("transaction_count", None),
                        "tps": record["_value"] if record["_field"] == "tps" else None,
                    })

            if data_points:
                df = pd.DataFrame(data_points)
                df["time"] = pd.to_datetime(df["time"])
                return df.sort_values("time")

            return pd.DataFrame()

        except Exception as e:
            st.error(f"Error fetching data: {e}")
            return pd.DataFrame()

    def render_header(self):
        """Render dashboard header"""
        col1, col2 = st.columns([3, 1])
        with col1:
            st.title("⚡ X3 Chain TPS Dashboard")
            st.markdown("Real-time blockchain performance monitoring")
        with col2:
            if st.checkbox("Auto-refresh", value=True):
                st.write("🔄 Live")

    def render_metrics(self, df: pd.DataFrame):
        """Render key metrics"""
        st.markdown("### Key Metrics")

        if df.empty:
            st.warning("No data available. Ensure TPS tracker is running.")
            return

        col1, col2, col3, col4 = st.columns(4)

        with col1:
            current_tps = df["tps"].iloc[-1] if len(df) > 0 else 0
            st.metric(
                "Current TPS",
                f"{current_tps:.2f}",
                delta=f"{(current_tps - df['tps'].iloc[-2]):.2f}" if len(df) > 1 else None
            )

        with col2:
            avg_tps = df["tps"].mean()
            st.metric("Average TPS", f"{avg_tps:.2f}")

        with col3:
            peak_tps = df["tps"].max()
            st.metric("Peak TPS", f"{peak_tps:.2f}")

        with col4:
            latest_block = df["block_height"].iloc[-1] if len(df) > 0 else 0
            st.metric("Latest Block", f"#{int(latest_block):,}")

    def render_tps_chart(self, df: pd.DataFrame):
        """Render TPS trend chart"""
        if df.empty:
            return

        st.markdown("### TPS Over Time")

        fig = go.Figure()

        fig.add_trace(go.Scatter(
            x=df["time"],
            y=df["tps"],
            mode="lines+markers",
            name="TPS",
            line={"color": "rgb(102, 126, 234)", "width": 3},
            marker={"size": 6},
            fill="tozeroy",
            fillcolor="rgba(102, 126, 234, 0.2)"
        ))

        # Add moving average
        df["tps_ma"] = df["tps"].rolling(window=min(10, len(df))).mean()
        fig.add_trace(go.Scatter(
            x=df["time"],
            y=df["tps_ma"],
            mode="lines",
            name="Moving Average (10s)",
            line={"color": "rgb(118, 75, 162)", "width": 2, "dash": "dash"}
        ))

        fig.update_layout(
            title="Transactions Per Second Trend",
            xaxis_title="Time",
            yaxis_title="TPS",
            hovermode="x unified",
            height=400,
            template="plotly_dark"
        )

        st.plotly_chart(fig, use_container_width=True)

    def render_block_distribution(self, df: pd.DataFrame):
        """Render block metrics"""
        if df.empty:
            return

        st.markdown("### Block Metrics")

        col1, col2 = st.columns(2)

        with col1:
            fig = go.Figure()
            fig.add_trace(go.Scatter(
                x=df["time"],
                y=df["transaction_count"],
                mode="lines+markers",
                name="Transactions per Block",
                line={"color": "rgb(255, 107, 107)", "width": 2},
                fill="tozeroy"
            ))
            fig.update_layout(
                title="Transactions per Block",
                xaxis_title="Time",
                yaxis_title="Count",
                height=350,
                template="plotly_dark"
            )
            st.plotly_chart(fig, use_container_width=True)

        with col2:
            # Block history
            st.markdown("#### Recent Blocks")
            recent = df.tail(10)[["time", "block_height", "transaction_count", "tps"]].copy()
            recent = recent.rename(columns={
                "time": "Time",
                "block_height": "Block",
                "transaction_count": "Txs",
                "tps": "TPS"
            })
            recent["Time"] = recent["Time"].dt.strftime("%H:%M:%S")
            st.dataframe(recent, use_container_width=True, hide_index=True)

    def render_statistics(self, df: pd.DataFrame):
        """Render statistical summary"""
        if df.empty:
            return

        st.markdown("### Statistics")

        col1, col2, col3 = st.columns(3)

        with col1:
            st.markdown("#### TPS Stats")
            tps_stats = df["tps"].describe()
            st.write(f"**Min:** {tps_stats['min']:.2f}")
            st.write(f"**Max:** {tps_stats['max']:.2f}")
            st.write(f"**Std Dev:** {tps_stats['std']:.2f}")
            st.write(f"**25th %ile:** {tps_stats['25%']:.2f}")
            st.write(f"**75th %ile:** {tps_stats['75%']:.2f}")

        with col2:
            st.markdown("#### Block Stats")
            st.write(f"**Total Blocks:** {int(df['block_height'].max() - df['block_height'].min()):,}")
            st.write(f"**Avg Txs/Block:** {df['transaction_count'].mean():.0f}")
            st.write(f"**Total Txs:** {df['transaction_count'].sum():,.0f}")

        with col3:
            st.markdown("#### Time Window")
            time_range = (df["time"].iloc[-1] - df["time"].iloc[0]).total_seconds()
            st.write(f"**Duration:** {time_range:.0f}s")
            st.write(f"**Data Points:** {len(df)}")
            st.write("**Sample Rate:** ~1/sec")

    def render(self):
        """Render complete dashboard"""
        self.render_header()

        # Sidebar controls
        st.sidebar.markdown("### Controls")
        time_range = st.sidebar.selectbox(
            "Time Range",
            ["15m", "1h", "6h", "24h"],
            index=1
        )

        refresh_interval = st.sidebar.slider(
            "Refresh Interval (seconds)",
            min_value=1,
            max_value=60,
            value=5
        )

        st.sidebar.markdown("### Configuration")
        st.sidebar.write(f"**InfluxDB URL:** {self.influx_url}")
        st.sidebar.write(f"**Database:** {self.influx_db}")
        st.sidebar.write(f"**Status:** {'🟢 Connected' if st.session_state.get('db_connected') else '🔴 Disconnected'}")

        # Fetch and render data
        df = self.fetch_data(time_range)

        self.render_metrics(df)
        self.render_tps_chart(df)
        self.render_block_distribution(df)
        self.render_statistics(df)

        # Auto-refresh
        if st.session_state.get("db_connected"):
            placeholder = st.empty()
            placeholder.metric("Last Update", datetime.now().strftime("%Y-%m-%d %H:%M:%S"))
            time.sleep(refresh_interval)
            st.rerun()


def main():
    """Main entry point"""
    if "db_connected" not in st.session_state:
        st.session_state.db_connected = False

    dashboard = TPSDashboard()
    dashboard.render()


if __name__ == "__main__":
    main()

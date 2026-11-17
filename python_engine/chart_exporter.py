"""
Módulo de Exportação de Gráficos Interativos (Plotly HTML) e Estáticos (Matplotlib PNG).
Permite que o usuário gere relatórios visuais detalhados a partir da interface em Rust.
"""

from pathlib import Path
import os
from typing import Any

import pandas as pd

from .cache_manager import DATA_DIR
from .stocks_service import consultar_acao_detalhada

# Garante diretório gravável local para cache do Matplotlib no Windows 11
MPL_CACHE_DIR = DATA_DIR / "mpl_cache"
MPL_CACHE_DIR.mkdir(parents=True, exist_ok=True)
os.environ.setdefault("MPLCONFIGDIR", str(MPL_CACHE_DIR))

try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
except ImportError:
    plt = None

try:
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots
except ImportError:
    go = None
    make_subplots = None

EXPORTS_DIR = DATA_DIR / "exports"


def exportar_grafico_acao(
    ticker: str,
    period: str = "1y",
    moeda: str = "ORIGINAL",
    formato: str = "plotly",
    start_date: str = "",
    end_date: str = "",
) -> dict[str, Any]:
    """
    Generates an interactive (Plotly HTML) or high-resolution (Matplotlib PNG) chart
    with Candlesticks, Moving Averages (SMA 20 and SMA 50), and Volume for the selected asset.
    """
    EXPORTS_DIR.mkdir(parents=True, exist_ok=True)
    dados = consultar_acao_detalhada(
        ticker=ticker,
        period=period,
        moeda_exibicao=moeda,
        usar_cache=True,
        start_date=start_date,
        end_date=end_date,
    )
    candles = dados.get("candles", [])
    if not candles:
        return {"status": "error", "message": "No candlestick data available to export."}

    df = pd.DataFrame(candles)
    short_code = dados.get("short_code", ticker)
    company = dados.get("company_name", ticker)
    curr = dados.get("display_currency", "BRL")

    if formato.lower() == "plotly" and go is not None and make_subplots is not None:
        fig = make_subplots(
            rows=2,
            cols=1,
            shared_xaxes=True,
            vertical_spacing=0.06,
            subplot_titles=(
                f"{short_code} ({company}) — Price & Moving Averages ({curr})",
                "Trading Volume",
            ),
            row_width=[0.25, 0.75],
        )

        fig.add_trace(
            go.Candlestick(
                x=df["date"],
                open=df["open"],
                high=df["high"],
                low=df["low"],
                close=df["close"],
                name="OHLC",
                increasing_line_color="#00E676",
                decreasing_line_color="#FF3D71",
            ),
            row=1,
            col=1,
        )

        fig.add_trace(
            go.Scatter(
                x=df["date"],
                y=df["sma20"],
                mode="lines",
                name="SMA 20",
                line={"color": "#38BDF8", "width": 1.6},
            ),
            row=1,
            col=1,
        )

        fig.add_trace(
            go.Scatter(
                x=df["date"],
                y=df["sma50"],
                mode="lines",
                name="SMA 50",
                line={"color": "#F59E0B", "width": 1.6},
            ),
            row=1,
            col=1,
        )

        fig.add_trace(
            go.Bar(
                x=df["date"],
                y=df["volume"],
                name="Volume",
                marker_color="#475569",
            ),
            row=2,
            col=1,
        )

        fig.update_layout(
            template="plotly_dark",
            title=f"Technical & Quantitative Analysis — {short_code} ({period.upper()})",
            xaxis_rangeslider_visible=False,
            paper_bgcolor="#0F172A",
            plot_bgcolor="#1E293B",
            height=760,
        )

        arquivo_html = EXPORTS_DIR / f"report_{short_code}_{period}.html"
        fig.write_html(str(arquivo_html), include_plotlyjs="cdn")
        return {
            "status": "ok",
            "format": "plotly_html",
            "file_path": str(arquivo_html.resolve()),
            "message": f"Interactive Plotly chart saved to {arquivo_html.name}",
        }

    if plt is not None:
        fig, (ax1, ax2) = plt.subplots(
            2, 1, figsize=(12, 7), gridspec_kw={"height_ratios": [3, 1]}, sharex=True
        )
        fig.patch.set_facecolor("#0F172A")
        for ax in (ax1, ax2):
            ax.set_facecolor("#1E293B")
            ax.tick_params(colors="#E2E8F0")
            ax.grid(True, linestyle="--", alpha=0.25, color="#94A3B8")

        ax1.plot(df["date"], df["close"], label=f"Close ({curr})", color="#10B981", linewidth=2.0)
        ax1.plot(df["date"], df["sma20"], label="SMA 20", color="#38BDF8", linewidth=1.3)
        ax1.plot(df["date"], df["sma50"], label="SMA 50", color="#F59E0B", linewidth=1.3)
        ax1.fill_between(df["date"], df["close"], df["close"].min(), color="#10B981", alpha=0.12)
        ax1.set_title(
            f"{short_code} - {company} ({period.upper()}) in {curr}",
            color="#F8FAFC",
            fontsize=13,
            fontweight="bold",
        )
        ax1.legend(facecolor="#0F172A", edgecolor="#334155", labelcolor="#F8FAFC")

        ax2.bar(df["date"], df["volume"], color="#3B82F6", alpha=0.65)
        ax2.set_ylabel("Volume", color="#E2E8F0")

        # Reduce X-axis labels to avoid overlap
        passo = max(1, len(df) // 10)
        ax2.set_xticks(range(0, len(df), passo))
        ax2.set_xticklabels(df["date"].iloc[::passo], rotation=30, ha="right")

        plt.tight_layout()
        arquivo_png = EXPORTS_DIR / f"chart_{short_code}_{period}.png"
        plt.savefig(arquivo_png, dpi=150, facecolor=fig.get_facecolor())
        plt.close(fig)
        return {
            "status": "ok",
            "format": "matplotlib_png",
            "file_path": str(arquivo_png.resolve()),
            "message": f"Matplotlib chart saved to {arquivo_png.name}",
        }

    return {
        "status": "error",
        "message": "Plotly/Matplotlib libraries not found in Python environment.",
    }


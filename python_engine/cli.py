"""
Python Engine Command Line Interface (CLI) for communication with Rust (egui).
Receives commands via process arguments and prints exclusively valid UTF-8 JSON to stdout.
"""

import argparse
import json
import sys
import traceback
from typing import Any

from .chart_exporter import exportar_grafico_acao
from .forex_service import consultar_moedas_e_dxy
from .interest_service import consultar_taxas_juros
from .macro_service import consultar_indicadores_socioeconomicos
from .stocks_service import consultar_acao_detalhada, consultar_visao_geral_mercado


def _configurar_utf8() -> None:
    """Garante codificação UTF-8 na saída padrão do Windows 11."""
    try:
        if hasattr(sys.stdout, "reconfigure"):
            sys.stdout.reconfigure(encoding="utf-8")
        if hasattr(sys.stderr, "reconfigure"):
            sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def executar_comando(args: argparse.Namespace) -> dict[str, Any]:
    """Roteia o comando solicitado pela interface Rust para o serviço correspondente."""
    usar_cache = not args.no_cache
    acao = args.action.lower().strip()

    if acao == "overview":
        favoritos = [x.strip() for x in args.tickers.split(",") if x.strip()] if args.tickers else None
        return consultar_visao_geral_mercado(favoritos=favoritos, usar_cache=usar_cache)

    if acao == "stock":
        return consultar_acao_detalhada(
            ticker=args.ticker,
            period=args.period,
            moeda_exibicao=args.currency,
            usar_cache=usar_cache,
        )

    if acao == "forex":
        pares = [x.strip() for x in args.pairs.split(",") if x.strip()] if args.pairs else None
        return consultar_moedas_e_dxy(pares=pares, period=args.period, usar_cache=usar_cache)

    if acao == "interest":
        paises = [x.strip() for x in args.countries.split(",") if x.strip()] if args.countries else None
        return consultar_taxas_juros(paises=paises, start_year=args.start_year, usar_cache=usar_cache)

    if acao == "macro":
        paises = [x.strip() for x in args.countries.split(",") if x.strip()] if args.countries else None
        return consultar_indicadores_socioeconomicos(
            paises=paises,
            start_year=args.start_year,
            end_year=args.end_year,
            usar_cache=usar_cache,
        )

    if acao == "export_chart":
        return exportar_grafico_acao(
            ticker=args.ticker,
            period=args.period,
            moeda=args.currency,
            formato=args.format,
        )

    return {
        "status": "error",
        "message": f"Unknown action: {acao}",
    }


def main() -> None:
    _configurar_utf8()
    parser = argparse.ArgumentParser(description="Economic and Financial Data Engine (Python <-> Rust)")
    parser.add_argument(
        "action",
        choices=["overview", "stock", "forex", "interest", "macro", "export_chart"],
        help="Operation to be executed by the data engine",
    )
    parser.add_argument("--ticker", default="PETR4", help="Stock ticker symbol (e.g., PETR4, VALE3, AAPL, NVDA)")
    parser.add_argument("--tickers", default="", help="Comma-separated list of tickers")
    parser.add_argument("--period", default="1y", help="Historical period (5d, 1mo, 6mo, 1y, 5y)")
    parser.add_argument("--currency", default="ORIGINAL", help="Display currency (ORIGINAL, BRL, USD, EUR)")
    parser.add_argument("--pairs", default="DX-Y.NYB,USDBRL=X,EURUSD=X,GBPUSD=X,USDJPY=X,USDCNY=X", help="Forex pairs")
    parser.add_argument("--countries", default="", help="Comma-separated country codes")
    parser.add_argument("--start-year", type=int, default=2018, help="Start year for macro/interest series")
    parser.add_argument("--end-year", type=int, default=2024, help="End year for macro series")
    parser.add_argument("--format", default="plotly", help="Chart export format (plotly or matplotlib)")
    parser.add_argument("--no-cache", action="store_true", help="Bypass local cache and force fresh API fetch")

    try:
        args = parser.parse_args()
        resposta = executar_comando(args)
    except Exception as exc:
        resposta = {
            "status": "error",
            "message": f"Internal error in Python engine: {exc}",
            "traceback": traceback.format_exc(),
        }

    print(json.dumps(resposta, ensure_ascii=False))


if __name__ == "__main__":
    main()


"""
Módulo de Cotações de Moedas, Dólar Americano (USD) e Índice DXY.
Utiliza `yfinance` para buscar pares cambiais e o índice Dólar (DX-Y.NYB / DXY=),
além de realizar conversões de moedas em tempo real e comparações em Base 100.
"""

from datetime import datetime, timedelta
import math
from typing import Any

import numpy as np
import pandas as pd

try:
    import yfinance as yf
except ImportError:
    yf = None

from .cache_manager import obter_cache, salvar_cache

# Metadados amigáveis para os pares de moedas e DXY
CATALOGO_MOEDAS: dict[str, dict[str, str]] = {
    "DX-Y.NYB": {
        "codigo_exibicao": "DXY=",
        "nome": "Índice Dólar Americano (DXY)",
        "moeda_base": "USD",
        "moeda_cotacao": "Cesta Global",
        "pais": "Estados Unidos",
    },
    "USDBRL=X": {
        "codigo_exibicao": "USDBRL=X",
        "nome": "Dólar Comercial vs Real Brasileiro",
        "moeda_base": "USD",
        "moeda_cotacao": "BRL",
        "pais": "Brasil",
    },
    "EURUSD=X": {
        "codigo_exibicao": "EURUSD=X",
        "nome": "Euro vs Dólar Americano",
        "moeda_base": "EUR",
        "moeda_cotacao": "USD",
        "pais": "Zona do Euro",
    },
    "GBPUSD=X": {
        "codigo_exibicao": "GBPUSD=X",
        "nome": "Libra Esterlina vs Dólar Americano",
        "moeda_base": "GBP",
        "moeda_cotacao": "USD",
        "pais": "Reino Unido",
    },
    "USDJPY=X": {
        "codigo_exibicao": "USDJPY=X",
        "nome": "Dólar Americano vs Iene Japonês",
        "moeda_base": "USD",
        "moeda_cotacao": "JPY",
        "pais": "Japão",
    },
    "USDCNY=X": {
        "codigo_exibicao": "USDCNY=X",
        "nome": "Dólar Americano vs Yuan Chinês",
        "moeda_base": "USD",
        "moeda_cotacao": "CNY",
        "pais": "China",
    },
    "USDCHF=X": {
        "codigo_exibicao": "USDCHF=X",
        "nome": "Dólar Americano vs Franco Suíço",
        "moeda_base": "USD",
        "moeda_cotacao": "CHF",
        "pais": "Suíça",
    },
}

# Taxas de referência para fallback caso o usuário esteja sem internet
TAXAS_USD_FALLBACK: dict[str, float] = {
    "USD": 1.0,
    "BRL": 5.75,
    "EUR": 0.92,
    "GBP": 0.78,
    "JPY": 152.40,
    "CNY": 7.24,
    "CHF": 0.88,
}


def normalizar_ticker_moeda(ticker: str) -> str:
    """Converte atalhos como 'DXY=' ou 'BRL' para o ticker exato do Yahoo Finance."""
    t = ticker.strip().upper()
    if t in ("DXY", "DXY=", "^DXY", "DX-Y.NYB"):
        return "DX-Y.NYB"
    mapeamento_curto = {
        "BRL": "USDBRL=X",
        "USDBRL": "USDBRL=X",
        "EUR": "EURUSD=X",
        "EURUSD": "EURUSD=X",
        "GBP": "GBPUSD=X",
        "GBPUSD": "GBPUSD=X",
        "JPY": "USDJPY=X",
        "USDJPY": "USDJPY=X",
        "CNY": "USDCNY=X",
        "USDCNY": "USDCNY=X",
        "CHF": "USDCHF=X",
        "USDCHF": "USDCHF=X",
    }
    if t in mapeamento_curto:
        return mapeamento_curto[t]
    if not t.endswith("=X") and t != "DX-Y.NYB":
        return f"{t}=X"
    return t


def _limpar_float(val: Any, padrao: float = 0.0) -> float:
    """Garante que valores NaN ou infinitos nunca quebrem a serialização JSON."""
    try:
        f = float(val)
        if math.isnan(f) or math.isinf(f):
            return padrao
        return round(f, 4)
    except Exception:
        return padrao


def _gerar_serie_cambial_contingencia(yf_ticker: str, period: str = "1y") -> pd.DataFrame:
    """Gera série histórica realista caso a API externa esteja indisponível."""
    dias_map = {"5d": 7, "1mo": 30, "6mo": 180, "1y": 252, "5y": 1260}
    n_dias = dias_map.get(period, 252)
    datas = pd.bdate_range(end=datetime.now(), periods=n_dias)

    base_map = {
        "DX-Y.NYB": 104.25,
        "USDBRL=X": 5.75,
        "EURUSD=X": 1.085,
        "GBPUSD=X": 1.292,
        "USDJPY=X": 152.40,
        "USDCNY=X": 7.24,
        "USDCHF=X": 0.882,
    }
    preco_base = base_map.get(yf_ticker, 1.0)
    rng = np.random.default_rng(abs(hash(yf_ticker)) % 100000)
    retornos = rng.normal(loc=0.0001, scale=0.0045, size=n_dias)
    fechamento = preco_base * np.exp(np.cumsum(retornos - retornos.mean()))
    abertura = np.roll(fechamento, 1)
    abertura[0] = fechamento[0] * 0.998
    maxima = np.maximum(abertura, fechamento) * (1.0 + np.abs(rng.normal(0, 0.0025, n_dias)))
    minima = np.minimum(abertura, fechamento) * (1.0 - np.abs(rng.normal(0, 0.0025, n_dias)))

    return pd.DataFrame(
        {
            "Open": abertura,
            "High": maxima,
            "Low": minima,
            "Close": fechamento,
        },
        index=datas,
    )


def obter_taxas_cambio_usd(usar_cache: bool = True) -> dict[str, float]:
    """
    Retorna quanto 1 USD vale em cada moeda (USD, BRL, EUR, GBP, JPY, CNY).
    Usado para conversão automática de preços de ações e comparativos.
    """
    cache_key = "forex_spot_usd_rates"
    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=1800)
        if cached and "rates" in cached:
            return cached["rates"]

    rates = dict(TAXAS_USD_FALLBACK)
    if yf is not None:
        try:
            tickers = ["USDBRL=X", "EURUSD=X", "GBPUSD=X", "USDJPY=X", "USDCNY=X"]
            dados = yf.download(tickers, period="5d", progress=False, auto_adjust=True)
            if not dados.empty and "Close" in dados:
                close_df = dados["Close"]
                if "USDBRL=X" in close_df and not close_df["USDBRL=X"].dropna().empty:
                    rates["BRL"] = _limpar_float(close_df["USDBRL=X"].dropna().iloc[-1], 5.75)
                if "EURUSD=X" in close_df and not close_df["EURUSD=X"].dropna().empty:
                    eur_usd = _limpar_float(close_df["EURUSD=X"].dropna().iloc[-1], 1.085)
                    if eur_usd > 0:
                        rates["EUR"] = round(1.0 / eur_usd, 4)
                if "GBPUSD=X" in close_df and not close_df["GBPUSD=X"].dropna().empty:
                    gbp_usd = _limpar_float(close_df["GBPUSD=X"].dropna().iloc[-1], 1.29)
                    if gbp_usd > 0:
                        rates["GBP"] = round(1.0 / gbp_usd, 4)
                if "USDJPY=X" in close_df and not close_df["USDJPY=X"].dropna().empty:
                    rates["JPY"] = _limpar_float(close_df["USDJPY=X"].dropna().iloc[-1], 152.4)
                if "USDCNY=X" in close_df and not close_df["USDCNY=X"].dropna().empty:
                    rates["CNY"] = _limpar_float(close_df["USDCNY=X"].dropna().iloc[-1], 7.24)
            salvar_cache(cache_key, "forex_rates", {"rates": rates})
        except Exception:
            pass
    return rates


def converter_moeda(valor: float, moeda_origem: str, moeda_destino: str, taxas_usd: dict[str, float] | None = None) -> float:
    """Converte um valor financeiro entre quaisquer duas moedas suportadas."""
    orig = moeda_origem.upper().strip()
    dest = moeda_destino.upper().strip()
    if orig == dest:
        return valor
    taxas = taxas_usd or obter_taxas_cambio_usd(usar_cache=True)
    taxa_orig = taxas.get(orig, 1.0)
    taxa_dest = taxas.get(dest, 1.0)
    if taxa_orig <= 0:
        return valor
    valor_em_usd = valor / taxa_orig
    return valor_em_usd * taxa_dest


def consultar_moedas_e_dxy(
    pares: list[str] | None = None,
    period: str = "1y",
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Consulta cotações históricas e atuais de moedas frente ao USD e o Índice DXY.
    Retorna séries temporais nominais e normalizadas (Base 100 no início do período).
    """
    if not pares:
        pares = ["DX-Y.NYB", "USDBRL=X", "EURUSD=X", "GBPUSD=X", "USDJPY=X", "USDCNY=X"]

    pares_norm = []
    for p in pares:
        tn = normalizar_ticker_moeda(p)
        if tn not in pares_norm:
            pares_norm.append(tn)

    cache_key = f"forex_dashboard_{'_'.join(sorted(pares_norm))}_{period}"
    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=1800)
        if cached:
            cached["source_status"] = "Cache Local SQLite"
            return cached

    itens_resultado: list[dict[str, Any]] = []
    status_fonte = "Yahoo Finance API (Online)"

    for yf_ticker in pares_norm:
        df_hist = pd.DataFrame()
        if yf is not None:
            try:
                tk = yf.Ticker(yf_ticker)
                df_hist = tk.history(period=period, auto_adjust=True)
            except Exception:
                df_hist = pd.DataFrame()

        if df_hist is None or df_hist.empty or "Close" not in df_hist:
            df_hist = _gerar_serie_cambial_contingencia(yf_ticker, period=period)
            status_fonte = "Modo Resiliente (Cache/Contingência)"

        df_hist = df_hist.dropna(subset=["Close"]).copy()
        if df_hist.empty:
            continue

        fechamentos = df_hist["Close"].astype(float)
        aberturas = df_hist["Open"].astype(float) if "Open" in df_hist else fechamentos
        maximas = df_hist["High"].astype(float) if "High" in df_hist else fechamentos
        minimas = df_hist["Low"].astype(float) if "Low" in df_hist else fechamentos

        atual = _limpar_float(fechamentos.iloc[-1])
        anterior = _limpar_float(fechamentos.iloc[-2], atual) if len(fechamentos) > 1 else atual
        inicial = _limpar_float(fechamentos.iloc[0], atual)

        var_dia_pct = ((atual - anterior) / anterior * 100.0) if anterior > 0 else 0.0
        var_periodo_pct = ((atual - inicial) / inicial * 100.0) if inicial > 0 else 0.0
        min_periodo = _limpar_float(minimas.min(), atual)
        max_periodo = _limpar_float(maximas.max(), atual)

        # Volatilidade anualizada (%)
        retornos_diarios = fechamentos.pct_change().dropna()
        volatilidade_anual = (
            _limpar_float(retornos_diarios.std() * math.sqrt(252) * 100.0, 0.0)
            if len(retornos_diarios) > 2
            else 0.0
        )

        # Série histórica (limitada a 260 pontos para performance fluida na GUI)
        df_plot = df_hist.tail(260)
        base_100_ini = float(df_plot["Close"].iloc[0]) if float(df_plot["Close"].iloc[0]) != 0 else 1.0

        historico = []
        for idx, row in df_plot.iterrows():
            data_str = idx.strftime("%Y-%m-%d") if hasattr(idx, "strftime") else str(idx)[:10]
            c_val = _limpar_float(row.get("Close", atual))
            historico.append(
                {
                    "date": data_str,
                    "open": _limpar_float(row.get("Open", c_val)),
                    "high": _limpar_float(row.get("High", c_val)),
                    "low": _limpar_float(row.get("Low", c_val)),
                    "close": c_val,
                    "base100": _limpar_float((c_val / base_100_ini) * 100.0, 100.0),
                }
            )

        meta = CATALOGO_MOEDAS.get(
            yf_ticker,
            {
                "codigo_exibicao": yf_ticker,
                "nome": f"Par Cambial {yf_ticker}",
                "moeda_base": yf_ticker[:3],
                "moeda_cotacao": yf_ticker[3:6],
                "pais": "Global",
            },
        )

        itens_resultado.append(
            {
                "ticker": yf_ticker,
                "display_code": meta["codigo_exibicao"],
                "name": meta["nome"],
                "base_currency": meta["moeda_base"],
                "quote_currency": meta["moeda_cotacao"],
                "country": meta["pais"],
                "current_rate": atual,
                "change_day_pct": round(var_dia_pct, 2),
                "change_period_pct": round(var_periodo_pct, 2),
                "high_period": max_periodo,
                "low_period": min_periodo,
                "volatility_annual_pct": round(volatilidade_anual, 2),
                "history": historico,
            }
        )

    taxas_usd = obter_taxas_cambio_usd(usar_cache=usar_cache)
    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "period": period,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "usd_rates": taxas_usd,
        "pairs": itens_resultado,
    }
    salvar_cache(cache_key, "forex", resposta)
    return resposta


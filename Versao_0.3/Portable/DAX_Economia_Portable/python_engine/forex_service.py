"""
Módulo de Cotações de Moedas, Dólar Americano (USD) e Índice DXY — DAX v0.2.
Utiliza `yfinance` para buscar até 10 pares cambiais e o índice Dólar (DX-Y.NYB / DXY=),
além de realizar conversões de moedas em tempo real e comparações em Base 100.
"""

from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import math
from typing import Any

import numpy as np
import pandas as pd

try:
    import yfinance as yf
except ImportError:
    yf = None

from .cache_manager import end_date_inclusivo, is_offline_mode, obter_cache, salvar_cache

# Catálogo expandido de pares cambiais e Índice Dólar (DXY) com vínculo aos países de Juros (BC) e World Bank
CATALOGO_MOEDAS: dict[str, dict[str, str]] = {
    "DX-Y.NYB": {
        "codigo_exibicao": "DXY=",
        "nome": "US Dollar Index (DXY)",
        "moeda_base": "USD",
        "moeda_cotacao": "Global Basket",
        "pais": "United States",
        "interest_code": "US",
        "wb_code": "USA",
    },
    "USDBRL=X": {
        "codigo_exibicao": "USDBRL=X",
        "nome": "US Dollar vs Brazilian Real (BRL)",
        "moeda_base": "USD",
        "moeda_cotacao": "BRL",
        "pais": "Brazil",
        "interest_code": "BR",
        "wb_code": "BRA",
    },
    "EURUSD=X": {
        "codigo_exibicao": "EURUSD=X",
        "nome": "Euro vs US Dollar (EUR)",
        "moeda_base": "EUR",
        "moeda_cotacao": "USD",
        "pais": "Eurozone / Germany",
        "interest_code": "EU",
        "wb_code": "DEU",
    },
    "GBPUSD=X": {
        "codigo_exibicao": "GBPUSD=X",
        "nome": "British Pound vs US Dollar (GBP)",
        "moeda_base": "GBP",
        "moeda_cotacao": "USD",
        "pais": "United Kingdom",
        "interest_code": "GB",
        "wb_code": "GBR",
    },
    "USDJPY=X": {
        "codigo_exibicao": "USDJPY=X",
        "nome": "US Dollar vs Japanese Yen (JPY)",
        "moeda_base": "USD",
        "moeda_cotacao": "JPY",
        "pais": "Japan",
        "interest_code": "JP",
        "wb_code": "JPN",
    },
    "USDCNY=X": {
        "codigo_exibicao": "USDCNY=X",
        "nome": "US Dollar vs Chinese Yuan (CNY)",
        "moeda_base": "USD",
        "moeda_cotacao": "CNY",
        "pais": "China",
        "interest_code": "CN",
        "wb_code": "CHN",
    },
    "USDCHF=X": {
        "codigo_exibicao": "USDCHF=X",
        "nome": "US Dollar vs Swiss Franc (CHF)",
        "moeda_base": "USD",
        "moeda_cotacao": "CHF",
        "pais": "Switzerland",
        "interest_code": "CH",
        "wb_code": "CHE",
    },
    "USDCAD=X": {
        "codigo_exibicao": "USDCAD=X",
        "nome": "US Dollar vs Canadian Dollar (CAD)",
        "moeda_base": "USD",
        "moeda_cotacao": "CAD",
        "pais": "Canada",
        "interest_code": "CA",
        "wb_code": "CAN",
    },
    "AUDUSD=X": {
        "codigo_exibicao": "AUDUSD=X",
        "nome": "Australian Dollar vs US Dollar (AUD)",
        "moeda_base": "AUD",
        "moeda_cotacao": "USD",
        "pais": "Australia",
        "interest_code": "AU",
        "wb_code": "AUS",
    },
    "USDMXN=X": {
        "codigo_exibicao": "USDMXN=X",
        "nome": "US Dollar vs Mexican Peso (MXN)",
        "moeda_base": "USD",
        "moeda_cotacao": "MXN",
        "pais": "Mexico",
        "interest_code": "MX",
        "wb_code": "MEX",
    },
    "USDARS=X": {
        "codigo_exibicao": "USDARS=X",
        "nome": "US Dollar vs Argentine Peso (ARS)",
        "moeda_base": "USD",
        "moeda_cotacao": "ARS",
        "pais": "Argentina",
        "interest_code": "AR",
        "wb_code": "ARG",
    },
    "USDINR=X": {
        "codigo_exibicao": "USDINR=X",
        "nome": "US Dollar vs Indian Rupee (INR)",
        "moeda_base": "USD",
        "moeda_cotacao": "INR",
        "pais": "India",
        "interest_code": "IN",
        "wb_code": "IND",
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
    "CAD": 1.39,
    "AUD": 1.52,
    "MXN": 20.15,
    "ARS": 995.00,
    "INR": 84.10,
}


def normalizar_ticker_moeda(ticker: str) -> str:
    """Converte atalhos como 'DXY=', 'BRL', 'CAD', 'ARS' para o ticker exato do Yahoo Finance."""
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
        "CAD": "USDCAD=X",
        "USDCAD": "USDCAD=X",
        "AUD": "AUDUSD=X",
        "AUDUSD": "AUDUSD=X",
        "USDAUD=X": "AUDUSD=X",
        "MXN": "USDMXN=X",
        "USDMXN": "USDMXN=X",
        "ARS": "USDARS=X",
        "USDARS": "USDARS=X",
        "INR": "USDINR=X",
        "USDINR": "USDINR=X",
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
        "USDCAD=X": 1.391,
        "AUDUSD=X": 0.658,
        "USDMXN=X": 20.15,
        "USDARS=X": 995.50,
        "USDINR=X": 84.10,
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
    Retorna quanto 1 USD vale em cada moeda suportada.
    Usado para conversão automática de preços de ações e comparativos.
    """
    cache_key = "forex_spot_usd_rates_v3"
    offline = is_offline_mode()
    if usar_cache or offline:
        cached = obter_cache(cache_key, ttl_seconds=600, permitir_expirado=offline)
        if cached and "rates" in cached:
            return cached["rates"]

    rates = dict(TAXAS_USD_FALLBACK)
    obteve_real = False
    if yf is not None and not offline:
        try:
            tickers = [
                "USDBRL=X", "EURUSD=X", "GBPUSD=X", "USDJPY=X", "USDCNY=X",
                "USDCHF=X", "USDCAD=X", "AUDUSD=X", "USDMXN=X", "USDARS=X", "USDINR=X",
            ]
            dados = yf.download(tickers, period="5d", progress=False, auto_adjust=True)
            if not dados.empty and "Close" in dados:
                close_df = dados["Close"]
                diretos = [
                    ("USDBRL=X", "BRL", 5.75),
                    ("USDJPY=X", "JPY", 152.4),
                    ("USDCNY=X", "CNY", 7.24),
                    ("USDCHF=X", "CHF", 0.88),
                    ("USDCAD=X", "CAD", 1.39),
                    ("USDMXN=X", "MXN", 20.15),
                    ("USDARS=X", "ARS", 995.0),
                    ("USDINR=X", "INR", 84.10),
                ]
                for tk, cod_m, pad in diretos:
                    if tk in close_df and not close_df[tk].dropna().empty:
                        rates[cod_m] = _limpar_float(close_df[tk].dropna().iloc[-1], pad)
                        obteve_real = True

                invertidos = [
                    ("EURUSD=X", "EUR", 1.085),
                    ("GBPUSD=X", "GBP", 1.29),
                    ("AUDUSD=X", "AUD", 0.658),
                ]
                for tk, cod_m, pad in invertidos:
                    if tk in close_df and not close_df[tk].dropna().empty:
                        val_usd = _limpar_float(close_df[tk].dropna().iloc[-1], pad)
                        if val_usd > 0:
                            rates[cod_m] = round(1.0 / val_usd, 4)
                            obteve_real = True

            if obteve_real:
                salvar_cache(cache_key, "forex_rates", {"rates": rates}, is_real=True)
        except Exception:
            pass

    if not obteve_real:
        last_real = obter_cache(cache_key, ttl_seconds=600, permitir_expirado=True, somente_real=True)
        if last_real and "rates" in last_real:
            return last_real["rates"]
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
    start_date: str = "",
    end_date: str = "",
) -> dict[str, Any]:
    """
    Queries historical and current quotes for up to 10 currencies vs USD and the DXY Index.
    Returns nominal and Base-100 normalized time series, supporting custom start/end dates.
    """
    if not pares:
        pares = ["DX-Y.NYB", "USDBRL=X", "EURUSD=X", "GBPUSD=X", "USDJPY=X", "USDCNY=X"]

    pares_norm = []
    for p in pares:
        tn = normalizar_ticker_moeda(p)
        if tn not in pares_norm:
            pares_norm.append(tn)

    # Limit to at most 10 currencies + DXY
    pares_norm = pares_norm[:11]

    has_custom_dates = bool(start_date.strip() and end_date.strip())
    range_key = f"{start_date.strip()}_{end_date.strip()}" if has_custom_dates else period
    cache_key = f"forex_dashboard_v3_{'_'.join(sorted(pares_norm))}_{range_key}"
    offline = is_offline_mode()
    if usar_cache or offline:
        cached = obter_cache(cache_key, ttl_seconds=600, permitir_expirado=offline)
        if cached:
            cached["source_status"] = "Offline Mode (Local Cache)" if offline else "Local SQLite Cache (10 min)"
            return cached

    itens_resultado: list[dict[str, Any]] = []
    status_fonte = "Yahoo Finance API (Online)"
    algum_par_real = False

    def _fetch_pair_hist(yf_tk: str) -> tuple[str, pd.DataFrame, bool]:
        df_h = pd.DataFrame()
        used_fallback = False
        if yf is not None and not offline:
            try:
                tk = yf.Ticker(yf_tk)
                if has_custom_dates:
                    df_h = tk.history(
                        start=start_date.strip(),
                        end=end_date_inclusivo(end_date.strip()),
                        auto_adjust=True,
                    )
                else:
                    df_h = tk.history(period=period, auto_adjust=True)
            except Exception:
                df_h = pd.DataFrame()

        if df_h is None or df_h.empty or "Close" not in df_h:
            df_h = _gerar_serie_cambial_contingencia(yf_tk, period="5y" if has_custom_dates else period)
            if has_custom_dates and not df_h.empty:
                idx_dates = df_h.index.strftime("%Y-%m-%d")
                mask = (idx_dates >= start_date.strip()) & (idx_dates <= end_date.strip())
                if mask.any():
                    df_h = df_h.loc[mask]
            used_fallback = True
        return yf_tk, df_h, used_fallback

    with ThreadPoolExecutor(max_workers=6) as executor:
        fetched_pairs = list(executor.map(_fetch_pair_hist, pares_norm))

    for yf_ticker, df_hist, used_fallback in fetched_pairs:
        if not used_fallback:
            algum_par_real = True
        elif not algum_par_real:
            status_fonte = "Offline Mode (Fallback)" if offline else "Resilient Mode (Cache/Fallback)"

    if not algum_par_real:
        last_real_fx = obter_cache(cache_key, ttl_seconds=600, permitir_expirado=True, somente_real=True)
        if last_real_fx:
            last_real_fx["source_status"] = "Last Verified Real Data (Local DB)"
            return last_real_fx

    for yf_ticker, df_hist, _used_fallback in fetched_pairs:

        df_hist = df_hist.dropna(subset=["Close"]).copy()
        if df_hist.empty:
            continue

        fechamentos = df_hist["Close"].astype(float)
        maximas = df_hist["High"].astype(float) if "High" in df_hist else fechamentos
        minimas = df_hist["Low"].astype(float) if "Low" in df_hist else fechamentos

        atual = _limpar_float(fechamentos.iloc[-1])
        anterior = _limpar_float(fechamentos.iloc[-2], atual) if len(fechamentos) > 1 else atual
        inicial = _limpar_float(fechamentos.iloc[0], atual)

        var_dia_pct = ((atual - anterior) / anterior * 100.0) if anterior > 0 else 0.0
        var_periodo_pct = ((atual - inicial) / inicial * 100.0) if inicial > 0 else 0.0
        min_periodo = _limpar_float(minimas.min(), atual)
        max_periodo = _limpar_float(maximas.max(), atual)

        retornos_diarios = fechamentos.pct_change().dropna()
        volatilidade_anual = (
            _limpar_float(retornos_diarios.std() * math.sqrt(252) * 100.0, 0.0)
            if len(retornos_diarios) > 2
            else 0.0
        )

        df_plot = df_hist if has_custom_dates else df_hist.tail(260)
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
                "nome": f"FX Pair {yf_ticker}",
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
    salvar_cache(cache_key, "forex", resposta, is_real=algum_par_real)
    return resposta

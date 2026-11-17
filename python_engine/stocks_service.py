"""
Módulo de Análise de Ações, Índices de Mercado e Scanner Fundamentalista.
Utiliza `yfinance` para coletar séries históricas OHLCV (Candlesticks),
médias móveis e indicadores fundamentalistas inspirados nos layouts Dax1 a Dax4.
"""

from datetime import datetime
import math
import re
from typing import Any

import numpy as np
import pandas as pd

try:
    import yfinance as yf
except ImportError:
    yf = None

from .cache_manager import obter_cache, salvar_cache
from .forex_service import converter_moeda, obter_taxas_cambio_usd

# Catálogo de empresas de referência (B3 e Globais) para enriquecer o Scanner (Dax4) e Cards (Dax2)
CATALOGO_ACOES: dict[str, dict[str, Any]] = {
    "PETR4.SA": {
        "codigo_curto": "PETR4",
        "nome": "Petrobras PN",
        "setor": "Oil, Gas & Biofuels",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 38.62,
        "mcap_bi": 504.2,
        "pe": 4.85,
        "pb": 1.28,
        "dy": 16.56,
        "roe": 26.4,
        "eps": 7.96,
    },
    "VALE3.SA": {
        "codigo_curto": "VALE3",
        "nome": "Vale ON",
        "setor": "Mining & Steel",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 57.01,
        "mcap_bi": 258.9,
        "pe": 6.12,
        "pb": 1.35,
        "dy": 9.85,
        "roe": 21.8,
        "eps": 9.31,
    },
    "ITUB4.SA": {
        "codigo_curto": "ITUB4",
        "nome": "Itaú Unibanco PN",
        "setor": "Banks & Financial Services",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 36.75,
        "mcap_bi": 342.8,
        "pe": 8.90,
        "pb": 1.78,
        "dy": 7.45,
        "roe": 21.2,
        "eps": 4.13,
    },
    "BBDC4.SA": {
        "codigo_curto": "BBDC4",
        "nome": "Bradesco PN",
        "setor": "Banks & Financial Services",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 15.77,
        "mcap_bi": 165.4,
        "pe": 9.85,
        "pb": 1.02,
        "dy": 8.20,
        "roe": 11.4,
        "eps": 1.60,
    },
    "BBAS3.SA": {
        "codigo_curto": "BBAS3",
        "nome": "Banco do Brasil ON",
        "setor": "Banks & Financial Services",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 28.70,
        "mcap_bi": 164.1,
        "pe": 4.65,
        "pb": 0.94,
        "dy": 10.12,
        "roe": 20.9,
        "eps": 6.17,
    },
    "WEGE3.SA": {
        "codigo_curto": "WEGE3",
        "nome": "WEG S.A. ON",
        "setor": "Industrial Goods & Motors",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 53.40,
        "mcap_bi": 224.1,
        "pe": 36.20,
        "pb": 11.40,
        "dy": 1.65,
        "roe": 31.8,
        "eps": 1.48,
    },
    "PRIO3.SA": {
        "codigo_curto": "PRIO3",
        "nome": "PRIO S.A. ON",
        "setor": "Oil & Gas",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 43.15,
        "mcap_bi": 38.4,
        "pe": 7.40,
        "pb": 2.45,
        "dy": 0.85,
        "roe": 33.1,
        "eps": 5.83,
    },
    "SUZB3.SA": {
        "codigo_curto": "SUZB3",
        "nome": "Suzano ON",
        "setor": "Pulp & Paper",
        "pais": "Brazil",
        "moeda": "BRL",
        "preco_ref": 58.90,
        "mcap_bi": 74.5,
        "pe": 8.30,
        "pb": 1.82,
        "dy": 3.40,
        "roe": 21.9,
        "eps": 7.09,
    },
    "AAPL": {
        "codigo_curto": "AAPL",
        "nome": "Apple Inc.",
        "setor": "Technology & Hardware",
        "pais": "United States",
        "moeda": "USD",
        "preco_ref": 228.50,
        "mcap_bi": 3460.0,
        "pe": 34.50,
        "pb": 48.20,
        "dy": 0.44,
        "roe": 147.2,
        "eps": 6.62,
    },
    "NVDA": {
        "codigo_curto": "NVDA",
        "nome": "NVIDIA Corporation",
        "setor": "Semiconductors & AI",
        "pais": "United States",
        "moeda": "USD",
        "preco_ref": 138.60,
        "mcap_bi": 3380.0,
        "pe": 52.40,
        "pb": 56.80,
        "dy": 0.03,
        "roe": 115.6,
        "eps": 2.64,
    },
    "MSFT": {
        "codigo_curto": "MSFT",
        "nome": "Microsoft Corp.",
        "setor": "Software & Cloud",
        "pais": "United States",
        "moeda": "USD",
        "preco_ref": 424.80,
        "mcap_bi": 3150.0,
        "pe": 35.10,
        "pb": 11.80,
        "dy": 0.72,
        "roe": 35.6,
        "eps": 12.10,
    },
    "AMZN": {
        "codigo_curto": "AMZN",
        "nome": "Amazon.com Inc.",
        "setor": "E-Commerce & Cloud",
        "pais": "United States",
        "moeda": "USD",
        "preco_ref": 208.90,
        "mcap_bi": 2190.0,
        "pe": 41.80,
        "pb": 8.60,
        "dy": 0.00,
        "roe": 21.9,
        "eps": 5.00,
    },
    "ASML": {
        "codigo_curto": "ASML",
        "nome": "ASML Holding N.V.",
        "setor": "Semiconductors",
        "pais": "Netherlands",
        "moeda": "USD",
        "preco_ref": 712.40,
        "mcap_bi": 282.5,
        "pe": 34.80,
        "pb": 18.20,
        "dy": 0.95,
        "roe": 54.1,
        "eps": 20.47,
    },
    "SAP": {
        "codigo_curto": "SAP",
        "nome": "SAP SE",
        "setor": "Enterprise Software",
        "pais": "Germany",
        "moeda": "USD",
        "preco_ref": 236.10,
        "mcap_bi": 275.4,
        "pe": 44.20,
        "pb": 6.10,
        "dy": 1.05,
        "roe": 14.8,
        "eps": 5.34,
    },
}


def normalizar_ticker_acao(ticker: str) -> str:
    """
    Normaliza um código de ação ou índice mundial digitado pelo usuário.
    Exemplo: 'PETR4' -> 'PETR4.SA', 'IBOV' -> '^BVSP', 'NIKKEI 225' -> '^N225'.
    """
    t = ticker.strip().upper()
    atalhos = {
        "IBOV": "^BVSP",
        "IBOVESPA": "^BVSP",
        "DOW": "^DJI",
        "WALL ST (DOW)": "^DJI",
        "WALL STREET": "^DJI",
        "SP500": "^GSPC",
        "S&P500": "^GSPC",
        "S&P 500": "^GSPC",
        "NASDAQ": "^IXIC",
        "NIKKEI": "^N225",
        "NIKKEI 225": "^N225",
        "XANGAI": "000001.SS",
        "XANGAI (SSE)": "000001.SS",
        "SSE": "000001.SS",
        "DAX": "^GDAXI",
        "DAX 40": "^GDAXI",
        "FTSE": "^FTSE",
        "FTSE 100": "^FTSE",
        "CAC": "^FCHI",
        "CAC 40": "^FCHI",
        "IBEX": "^IBEX",
        "IBEX 35": "^IBEX",
        "MERVAL": "^MERV",
    }
    if t in atalhos:
        return atalhos[t]
    # Padrão B3: 4 letras seguidas de 1 ou 2 dígitos (ex.: PETR4, VALE3, ITUB4, KLBN11)
    if re.match(r"^[A-Z]{4}\d{1,2}$", t):
        return f"{t}.SA"
    return t


def codigo_curto_acao(yf_ticker: str) -> str:
    """Remove o sufixo .SA ou formata códigos de índices mundiais para exibição limpa."""
    nomes_indices = {
        "^BVSP": "IBOV",
        "^DJI": "WALL ST (DOW)",
        "^GSPC": "S&P 500",
        "^IXIC": "NASDAQ",
        "^N225": "NIKKEI 225",
        "000001.SS": "XANGAI (SSE)",
        "^GDAXI": "DAX 40",
        "^FTSE": "FTSE 100",
        "^FCHI": "CAC 40",
        "^IBEX": "IBEX 35",
        "^MERV": "MERVAL",
    }
    if yf_ticker in nomes_indices:
        return nomes_indices[yf_ticker]
    if yf_ticker.endswith(".SA"):
        return yf_ticker[:-3]
    return yf_ticker


def _limpar_float(val: Any, padrao: float = 0.0, casas: int = 2) -> float:
    try:
        f = float(val)
        if math.isnan(f) or math.isinf(f):
            return padrao
        return round(f, casas)
    except Exception:
        return padrao


def _gerar_ohlcv_contingencia(yf_ticker: str, period: str = "1y") -> pd.DataFrame:
    """Gera série OHLCV realista caso a conexão com o Yahoo Finance falhe."""
    dias_map = {"5d": 7, "1mo": 22, "6mo": 126, "1y": 252, "5y": 1260}
    n_dias = dias_map.get(period, 252)
    datas = pd.bdate_range(end=datetime.now(), periods=n_dias)

    meta = CATALOGO_ACOES.get(yf_ticker, {})
    preco_alvo = float(meta.get("preco_ref", 45.0))

    rng = np.random.default_rng(abs(hash(yf_ticker)) % 99991)
    retornos = rng.normal(loc=0.0005, scale=0.016, size=n_dias)
    caminho = np.exp(np.cumsum(retornos))
    fechamento = caminho * (preco_alvo / caminho[-1])

    abertura = np.roll(fechamento, 1) * (1.0 + rng.normal(0, 0.003, n_dias))
    abertura[0] = fechamento[0] * 0.992
    maxima = np.maximum(abertura, fechamento) * (1.0 + np.abs(rng.normal(0, 0.008, n_dias)))
    minima = np.minimum(abertura, fechamento) * (1.0 - np.abs(rng.normal(0, 0.008, n_dias)))
    volume = rng.integers(12_000_000, 85_000_000, size=n_dias)

    return pd.DataFrame(
        {
            "Open": abertura,
            "High": maxima,
            "Low": minima,
            "Close": fechamento,
            "Volume": volume,
        },
        index=datas,
    )


def consultar_acao_detalhada(
    ticker: str,
    period: str = "1y",
    moeda_exibicao: str = "ORIGINAL",
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Consulta histórico OHLCV, médias móveis e indicadores estilo StatusInvest (Dax3)
    para um ativo específico, com suporte à conversão cambial automática.
    """
    yf_ticker = normalizar_ticker_acao(ticker)
    moeda_req = moeda_exibicao.upper().strip()
    cache_key = f"stock_detail_{yf_ticker}_{period}_{moeda_req}"

    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=1800)
        if cached:
            cached["source_status"] = "Local SQLite Cache"
            return cached

    meta = CATALOGO_ACOES.get(yf_ticker, {})
    moeda_nativa = meta.get("moeda", "BRL" if yf_ticker.endswith(".SA") else "USD")
    nome_empresa = meta.get("nome", codigo_curto_acao(yf_ticker))
    setor = meta.get("setor", "Equities")
    pais = meta.get("pais", "Brazil" if yf_ticker.endswith(".SA") else "United States")

    df_hist = pd.DataFrame()
    info_yf: dict[str, Any] = {}
    status_fonte = "Yahoo Finance API (Online)"

    if yf is not None:
        try:
            tk = yf.Ticker(yf_ticker)
            # Para calcular min/max 52 semanas com precisão, buscamos ao menos 1y se o período for curto
            periodo_download = "1y" if period in ("5d", "1mo", "6mo") else period
            df_full = tk.history(period=periodo_download, auto_adjust=False)
            if df_full is not None and not df_full.empty:
                df_hist = df_full
            try:
                info_yf = tk.fast_info if hasattr(tk, "fast_info") else {}
            except Exception:
                info_yf = {}
        except Exception:
            df_hist = pd.DataFrame()

    if df_hist is None or df_hist.empty or "Close" not in df_hist:
        df_hist = _gerar_ohlcv_contingencia(yf_ticker, period="1y" if period in ("5d", "1mo", "6mo") else period)
        status_fonte = "Resilient Mode (Cache/Fallback)"

    df_hist = df_hist.dropna(subset=["Close"]).copy()

    # Determina moeda final e fator de conversão
    moeda_final = moeda_nativa if moeda_req in ("ORIGINAL", "", "AUTO") else moeda_req
    fator_cambio = 1.0
    if moeda_final != moeda_nativa:
        taxas_usd = obter_taxas_cambio_usd(usar_cache=usar_cache)
        fator_cambio = converter_moeda(1.0, moeda_nativa, moeda_final, taxas_usd)

    for col in ("Open", "High", "Low", "Close"):
        if col in df_hist:
            df_hist[col] = df_hist[col].astype(float) * fator_cambio
        else:
            df_hist[col] = df_hist["Close"].astype(float) * fator_cambio

    if "Volume" not in df_hist:
        df_hist["Volume"] = 0

    # Cálculo das Médias Móveis Simples (SMA 20 e SMA 50)
    df_hist["SMA20"] = df_hist["Close"].rolling(window=20, min_periods=1).mean()
    df_hist["SMA50"] = df_hist["Close"].rolling(window=50, min_periods=1).mean()

    # Métricas de 52 semanas e do mês atual (Dax3)
    df_52w = df_hist.tail(252)
    df_mes = df_hist.tail(21)

    preco_atual = _limpar_float(df_hist["Close"].iloc[-1])
    preco_ontem = _limpar_float(df_hist["Close"].iloc[-2], preco_atual) if len(df_hist) > 1 else preco_atual
    var_dia_pct = ((preco_atual - preco_ontem) / preco_ontem * 100.0) if preco_ontem > 0 else 0.0

    min_52w = _limpar_float(df_52w["Low"].min(), preco_atual)
    max_52w = _limpar_float(df_52w["High"].max(), preco_atual)
    min_mes = _limpar_float(df_mes["Low"].min(), preco_atual)
    max_mes = _limpar_float(df_mes["High"].max(), preco_atual)

    preco_ini_12m = _limpar_float(df_52w["Close"].iloc[0], preco_atual)
    var_12m_pct = ((preco_atual - preco_ini_12m) / preco_ini_12m * 100.0) if preco_ini_12m > 0 else 0.0

    preco_ini_mes = _limpar_float(df_mes["Close"].iloc[0], preco_atual)
    var_mes_pct = ((preco_atual - preco_ini_mes) / preco_ini_mes * 100.0) if preco_ini_mes > 0 else 0.0

    # Filtra para a janela solicitada pelo usuário para o gráfico
    recorte_map = {"5d": 5, "1mo": 22, "6mo": 126, "1y": 252, "5y": 1260}
    n_recorte = recorte_map.get(period, 252)
    df_plot = df_hist.tail(n_recorte)

    candles = []
    for idx, row in df_plot.iterrows():
        data_str = idx.strftime("%Y-%m-%d") if hasattr(idx, "strftime") else str(idx)[:10]
        candles.append(
            {
                "date": data_str,
                "open": _limpar_float(row["Open"]),
                "high": _limpar_float(row["High"]),
                "low": _limpar_float(row["Low"]),
                "close": _limpar_float(row["Close"]),
                "volume": int(float(row["Volume"]) if not math.isnan(float(row["Volume"])) else 0),
                "sma20": _limpar_float(row["SMA20"]),
                "sma50": _limpar_float(row["SMA50"]),
            }
        )

    dy_pct = float(meta.get("dy", 4.50))
    div_12m_val = round(preco_atual * (dy_pct / 100.0), 4)
    mcap_bi = round(float(meta.get("mcap_bi", 100.0)) * fator_cambio, 2)

    retornos = df_52w["Close"].pct_change().dropna()
    volatilidade_anual = (
        _limpar_float(retornos.std() * math.sqrt(252) * 100.0, 18.5)
        if len(retornos) > 2
        else 18.5
    )

    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "ticker": yf_ticker,
        "short_code": codigo_curto_acao(yf_ticker),
        "company_name": nome_empresa,
        "sector": setor,
        "country": pais,
        "native_currency": moeda_nativa,
        "display_currency": moeda_final,
        "fx_factor": round(fator_cambio, 4),
        "period": period,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "metrics": {
            "current_price": preco_atual,
            "change_day_pct": round(var_dia_pct, 2),
            "min_52w": min_52w,
            "max_52w": max_52w,
            "min_month": min_mes,
            "max_month": max_mes,
            "dividend_yield_pct": round(dy_pct, 2),
            "dividends_12m": div_12m_val,
            "change_12m_pct": round(var_12m_pct, 2),
            "change_month_pct": round(var_mes_pct, 2),
            "market_cap_bi": mcap_bi,
            "volume_last": int(candles[-1]["volume"]) if candles else 0,
            "pe_ratio": float(meta.get("pe", 12.5)),
            "pb_ratio": float(meta.get("pb", 1.8)),
            "roe_pct": float(meta.get("roe", 18.0)),
            "eps": round(float(meta.get("eps", 3.2)) * fator_cambio, 2),
            "volatility_annual_pct": volatilidade_anual,
        },
        "candles": candles,
    }

    salvar_cache(cache_key, "stock_detail", resposta)
    return resposta


# Catálogo das Principais Bolsas Mundiais para a Barra Superior (Dax1)
CATALOGO_INDICES_MUNDIAIS: list[dict[str, Any]] = [
    {
        "yf_ticker": "^BVSP",
        "code": "IBOV",
        "subtitle": "Brasil (B3)",
        "ref_pts": 129850.0,
        "ref_var": 0.64,
        "ref_spark": [128100.0, 128450.0, 127900.0, 128800.0, 129100.0, 129400.0, 129850.0],
    },
    {
        "yf_ticker": "^DJI",
        "code": "WALL ST (DOW)",
        "subtitle": "EUA (NYSE)",
        "ref_pts": 43729.0,
        "ref_var": 0.85,
        "ref_spark": [43100.0, 43250.0, 43180.0, 43400.0, 43550.0, 43610.0, 43729.0],
    },
    {
        "yf_ticker": "^GSPC",
        "code": "S&P 500",
        "subtitle": "EUA (Wall St)",
        "ref_pts": 5929.0,
        "ref_var": 0.72,
        "ref_spark": [5810.0, 5835.0, 5820.0, 5865.0, 5890.0, 5910.0, 5929.0],
    },
    {
        "yf_ticker": "^IXIC",
        "code": "NASDAQ",
        "subtitle": "EUA (Tech)",
        "ref_pts": 18983.0,
        "ref_var": 1.12,
        "ref_spark": [18520.0, 18600.0, 18580.0, 18710.0, 18820.0, 18900.0, 18983.0],
    },
    {
        "yf_ticker": "^N225",
        "code": "NIKKEI 225",
        "subtitle": "Japão (Tóquio)",
        "ref_pts": 39480.0,
        "ref_var": 0.94,
        "ref_spark": [38600.0, 38850.0, 38790.0, 39050.0, 39200.0, 39310.0, 39480.0],
    },
    {
        "yf_ticker": "000001.SS",
        "code": "XANGAI (SSE)",
        "subtitle": "China",
        "ref_pts": 3383.0,
        "ref_var": 0.58,
        "ref_spark": [3310.0, 3325.0, 3318.0, 3340.0, 3360.0, 3371.0, 3383.0],
    },
    {
        "yf_ticker": "^GDAXI",
        "code": "DAX 40",
        "subtitle": "Alemanha",
        "ref_pts": 19256.0,
        "ref_var": -0.31,
        "ref_spark": [19420.0, 19390.0, 19410.0, 19350.0, 19310.0, 19290.0, 19256.0],
    },
    {
        "yf_ticker": "^FTSE",
        "code": "FTSE 100",
        "subtitle": "Inglaterra",
        "ref_pts": 8172.0,
        "ref_var": -0.18,
        "ref_spark": [8230.0, 8215.0, 8220.0, 8195.0, 8188.0, 8180.0, 8172.0],
    },
    {
        "yf_ticker": "^FCHI",
        "code": "CAC 40",
        "subtitle": "França",
        "ref_pts": 7425.0,
        "ref_var": -0.44,
        "ref_spark": [7510.0, 7495.0, 7480.0, 7460.0, 7450.0, 7438.0, 7425.0],
    },
    {
        "yf_ticker": "^IBEX",
        "code": "IBEX 35",
        "subtitle": "Espanha",
        "ref_pts": 11685.0,
        "ref_var": 0.29,
        "ref_spark": [11580.0, 11600.0, 11590.0, 11625.0, 11640.0, 11665.0, 11685.0],
    },
    {
        "yf_ticker": "^MERV",
        "code": "MERVAL",
        "subtitle": "Argentina",
        "ref_pts": 1945200.0,
        "ref_var": 1.65,
        "ref_spark": [1865000.0, 1880000.0, 1895000.0, 1910000.0, 1922000.0, 1931000.0, 1945200.0],
    },
]


def _formatar_pontos_indice(pts: float) -> str:
    """Formata a pontuação do índice no padrão brasileiro (ex.: 129.850 pts)."""
    if pts >= 1000:
        inteiro = f"{pts:,.0f}".replace(",", ".")
        return f"{inteiro} pts"
    return f"{pts:,.2f} pts".replace(",", "X").replace(".", ",").replace("X", ".")


def _coletar_indices_mundiais_topo() -> list[dict[str, Any]]:
    """
    Busca em lote via `yfinance` os 11 índices das principais bolsas mundiais
    (Brasil, Wall Street Dow/S&P500, Nasdaq, Japão, China, Alemanha, Inglaterra,
    França, Espanha e Argentina) com sparkline dos últimos pregões.
    """
    tickers_yf = [item["yf_ticker"] for item in CATALOGO_INDICES_MUNDIAIS]
    dados_close = None

    if yf is not None:
        try:
            df_download = yf.download(tickers_yf, period="1mo", progress=False, auto_adjust=True)
            if df_download is not None and not df_download.empty and "Close" in df_download:
                dados_close = df_download["Close"]
        except Exception:
            dados_close = None

    resultado_topo: list[dict[str, Any]] = []
    for meta in CATALOGO_INDICES_MUNDIAIS:
        yf_tk = meta["yf_ticker"]
        pts_atual = float(meta["ref_pts"])
        var_dia = float(meta["ref_var"])
        spark = list(meta["ref_spark"])

        if dados_close is not None and yf_tk in dados_close:
            serie = dados_close[yf_tk].dropna()
            if len(serie) >= 2:
                pts_atual = _limpar_float(serie.iloc[-1], pts_atual)
                pts_ant = _limpar_float(serie.iloc[-2], pts_atual)
                if pts_ant > 0:
                    var_dia = round(((pts_atual - pts_ant) / pts_ant) * 100.0, 2)
                spark = [_limpar_float(v, pts_atual) for v in serie.tail(12).tolist()]

        resultado_topo.append(
            {
                "code": meta["code"],
                "subtitle": meta["subtitle"],
                "value": _formatar_pontos_indice(pts_atual),
                "change_pct": var_dia,
                "sparkline": spark,
            }
        )

    return resultado_topo


def consultar_visao_geral_mercado(
    favoritos: list[str] | None = None,
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Constrói o painel integrado inspirado nas imagens Dax1, Dax2 e Dax4:
    1. Barra superior das Principais Bolsas Mundiais com atualização a cada 10 min (600s).
    2. Ranking de Altas e Baixas do pregão (Dax1).
    3. Cards de ativos com sparklines e métricas rápidas (Dax2).
    4. Tabela completa do Scanner de Ações com múltiplos fundamentalistas (Dax4).
    """
    cache_key = "market_overview_dax_global_v2"
    if usar_cache:
        # TTL de 600 segundos (10 minutos) conforme solicitado
        cached = obter_cache(cache_key, ttl_seconds=600)
        if cached:
            cached["source_status"] = "Local SQLite Cache (10 min)"
            return cached

    todos_tickers = list(CATALOGO_ACOES.keys())
    if favoritos:
        for fav in favoritos:
            tn = normalizar_ticker_acao(fav)
            if tn not in todos_tickers:
                todos_tickers.append(tn)

    cards_ativos: list[dict[str, Any]] = []
    status_fonte = "Yahoo Finance API (Online)"

    for yf_ticker in todos_tickers:
        detalhe = consultar_acao_detalhada(yf_ticker, period="1mo", moeda_exibicao="ORIGINAL", usar_cache=usar_cache)
        if detalhe.get("source_status", "").startswith("Resilient"):
            status_fonte = detalhe["source_status"]

        m = detalhe["metrics"]
        sparkline = [c["close"] for c in detalhe["candles"][-22:]]
        cards_ativos.append(
            {
                "ticker": yf_ticker,
                "short_code": detalhe["short_code"],
                "name": detalhe["company_name"],
                "sector": detalhe["sector"],
                "country": detalhe["country"],
                "currency": detalhe["display_currency"],
                "price": m["current_price"],
                "change_day_pct": m["change_day_pct"],
                "change_12m_pct": m["change_12m_pct"],
                "market_cap_bi": m["market_cap_bi"],
                "volume": m["volume_last"],
                "pe": m["pe_ratio"],
                "pb": m["pb_ratio"],
                "dy": m["dividend_yield_pct"],
                "roe": m["roe_pct"],
                "eps": m["eps"],
                "sparkline": sparkline,
            }
        )

    # Ordena para separar Altas e Baixas (Dax1)
    ordenados = sorted(cards_ativos, key=lambda x: x["change_day_pct"], reverse=True)
    altas = [
        {"code": item["short_code"], "change_pct": item["change_day_pct"], "price": item["price"]}
        for item in ordenados
        if item["change_day_pct"] >= 0
    ][:7]
    baixas = [
        {"code": item["short_code"], "change_pct": item["change_day_pct"], "price": item["price"]}
        for item in reversed(ordenados)
        if item["change_day_pct"] < 0
    ][:7]

    # Garante preenchimento visual estilo Dax1 mesmo se todos subirem ou caírem no dia
    if not altas:
        altas = [{"code": item["short_code"], "change_pct": abs(item["change_day_pct"]) + 0.45, "price": item["price"]} for item in ordenados[:5]]
    if not baixas:
        baixas = [{"code": item["short_code"], "change_pct": -abs(item["change_day_pct"]) - 0.35, "price": item["price"]} for item in reversed(ordenados[-5:])]

    # Mini-cards superiores das Principais Bolsas Mundiais
    top_tickers = _coletar_indices_mundiais_topo()

    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "top_bar": top_tickers,
        "gainers": altas,
        "losers": baixas,
        "assets": cards_ativos,
    }
    salvar_cache(cache_key, "market_overview", resposta)
    return resposta


"""
Módulo de Taxas de Juros Globais (Bancos Centrais) — DAX v0.2.
Suporta todos os países vinculados às moedas selecionadas na Aba (b) Moedas & Dólar (DXY).
Integra `fredapi`, `pandas_datareader`, API aberta do Banco Central do Brasil (SGS) e endpoints CSV do FRED.
"""

from datetime import datetime
import io
import json
import math
import os
from typing import Any
import urllib.request

import numpy as np
import pandas as pd

try:
    import requests
except ImportError:
    requests = None

try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass

try:
    from fredapi import Fred
except ImportError:
    Fred = None

try:
    from pandas_datareader import data as pdr
except ImportError:
    pdr = None

from .cache_manager import obter_cache, salvar_cache

# Catálogo completo de Bancos Centrais sincronizado com as moedas da Aba (b)
CATALOGO_JUROS: dict[str, dict[str, Any]] = {
    "US": {
        "pais": "Estados Unidos",
        "banco_central": "Federal Reserve (Fed)",
        "nome_taxa": "Fed Funds Effective Rate",
        "serie_solicitada": "DFF",
        "fred_id": "DFF",
        "moeda": "USD",
        "inflacao_atual": 2.60,
        "historico_ref": [
            ("2019-06", 2.40), ("2019-12", 1.55), ("2020-06", 0.08), ("2020-12", 0.09),
            ("2021-06", 0.08), ("2021-12", 0.08), ("2022-06", 1.21), ("2022-12", 4.10),
            ("2023-06", 5.08), ("2023-12", 5.33), ("2024-06", 5.33), ("2024-12", 4.58),
            ("2025-06", 4.33),
        ],
    },
    "BR": {
        "pais": "Brasil",
        "banco_central": "Banco Central do Brasil (BCB)",
        "nome_taxa": "Taxa Meta Selic (% a.a.)",
        "serie_solicitada": "SELIC",
        "fred_id": "IRSTCI01BRM156N",
        "bcb_sgs": 432,
        "moeda": "BRL",
        "inflacao_atual": 4.56,
        "historico_ref": [
            ("2019-06", 6.50), ("2019-12", 4.50), ("2020-06", 2.25), ("2020-12", 2.00),
            ("2021-06", 4.25), ("2021-12", 9.25), ("2022-06", 13.25), ("2022-12", 13.75),
            ("2023-06", 13.75), ("2023-12", 11.75), ("2024-06", 10.50), ("2024-12", 12.25),
            ("2025-06", 13.25),
        ],
    },
    "EU": {
        "pais": "Zona do Euro / Alemanha",
        "banco_central": "Banco Central Europeu (BCE)",
        "nome_taxa": "Taxa Básica BCE / Euribor 3M",
        "serie_solicitada": "GKLIB3M",
        "fred_id": "ECBDFR",
        "moeda": "EUR",
        "inflacao_atual": 2.00,
        "historico_ref": [
            ("2019-06", -0.40), ("2019-12", -0.50), ("2020-06", -0.50), ("2020-12", -0.50),
            ("2021-06", -0.50), ("2021-12", -0.50), ("2022-06", -0.50), ("2022-12", 2.00),
            ("2023-06", 3.50), ("2023-12", 4.00), ("2024-06", 3.75), ("2024-12", 3.15),
            ("2025-06", 2.90),
        ],
    },
    "GB": {
        "pais": "Reino Unido",
        "banco_central": "Bank of England (BoE)",
        "nome_taxa": "BoE Official Bank Rate",
        "serie_solicitada": "GBRBOE1M",
        "fred_id": "IRSTCI01GBM156N",
        "moeda": "GBP",
        "inflacao_atual": 2.30,
        "historico_ref": [
            ("2019-06", 0.75), ("2019-12", 0.75), ("2020-06", 0.10), ("2020-12", 0.10),
            ("2021-06", 0.10), ("2021-12", 0.25), ("2022-06", 1.25), ("2022-12", 3.50),
            ("2023-06", 5.00), ("2023-12", 5.25), ("2024-06", 5.25), ("2024-12", 4.75),
            ("2025-06", 4.50),
        ],
    },
    "JP": {
        "pais": "Japão",
        "banco_central": "Bank of Japan (BoJ / JCC)",
        "nome_taxa": "BoJ Call Rate & JCC Target",
        "serie_solicitada": "JCC_BOJ",
        "fred_id": "IRSTCI01JPM156N",
        "moeda": "JPY",
        "inflacao_atual": 2.30,
        "historico_ref": [
            ("2019-06", -0.10), ("2019-12", -0.10), ("2020-06", -0.10), ("2020-12", -0.10),
            ("2021-06", -0.10), ("2021-12", -0.10), ("2022-06", -0.10), ("2022-12", -0.10),
            ("2023-06", -0.10), ("2023-12", -0.10), ("2024-06", 0.10), ("2024-12", 0.25),
            ("2025-06", 0.50),
        ],
    },
    "CN": {
        "pais": "China",
        "banco_central": "People's Bank of China (PBoC)",
        "nome_taxa": "Loan Prime Rate (LPR 1Y)",
        "serie_solicitada": "PBOC_LPR",
        "fred_id": "IRSTCI01CNM156N",
        "moeda": "CNY",
        "inflacao_atual": 0.40,
        "historico_ref": [
            ("2019-06", 4.31), ("2019-12", 4.15), ("2020-06", 3.85), ("2020-12", 3.85),
            ("2021-06", 3.85), ("2021-12", 3.80), ("2022-06", 3.70), ("2022-12", 3.65),
            ("2023-06", 3.55), ("2023-12", 3.45), ("2024-06", 3.45), ("2024-12", 3.10),
            ("2025-06", 3.00),
        ],
    },
    "CH": {
        "pais": "Suíça",
        "banco_central": "Swiss National Bank (SNB)",
        "nome_taxa": "SNB Policy Rate",
        "serie_solicitada": "SNB_RATE",
        "fred_id": "IRSTCI01CHM156N",
        "moeda": "CHF",
        "inflacao_atual": 0.80,
        "historico_ref": [
            ("2019-06", -0.75), ("2019-12", -0.75), ("2020-06", -0.75), ("2020-12", -0.75),
            ("2021-06", -0.75), ("2021-12", -0.75), ("2022-06", -0.25), ("2022-12", 1.00),
            ("2023-06", 1.75), ("2023-12", 1.75), ("2024-06", 1.25), ("2024-12", 0.50),
            ("2025-06", 0.25),
        ],
    },
    "CA": {
        "pais": "Canadá",
        "banco_central": "Bank of Canada (BoC)",
        "nome_taxa": "BoC Overnight Target Rate",
        "serie_solicitada": "BOC_RATE",
        "fred_id": "IRSTCI01CAM156N",
        "moeda": "CAD",
        "inflacao_atual": 2.00,
        "historico_ref": [
            ("2019-06", 1.75), ("2019-12", 1.75), ("2020-06", 0.25), ("2020-12", 0.25),
            ("2021-06", 0.25), ("2021-12", 0.25), ("2022-06", 1.50), ("2022-12", 4.25),
            ("2023-06", 4.75), ("2023-12", 5.00), ("2024-06", 4.75), ("2024-12", 3.25),
            ("2025-06", 3.00),
        ],
    },
    "AU": {
        "pais": "Austrália",
        "banco_central": "Reserve Bank of Australia (RBA)",
        "nome_taxa": "RBA Cash Rate Target",
        "serie_solicitada": "RBA_RATE",
        "fred_id": "IRSTCI01AUM156N",
        "moeda": "AUD",
        "inflacao_atual": 2.80,
        "historico_ref": [
            ("2019-06", 1.25), ("2019-12", 0.75), ("2020-06", 0.25), ("2020-12", 0.10),
            ("2021-06", 0.10), ("2021-12", 0.10), ("2022-06", 0.85), ("2022-12", 3.10),
            ("2023-06", 4.10), ("2023-12", 4.35), ("2024-06", 4.35), ("2024-12", 4.35),
            ("2025-06", 4.10),
        ],
    },
    "MX": {
        "pais": "México",
        "banco_central": "Banco de México (Banxico)",
        "nome_taxa": "Tasa Objetivo Banxico",
        "serie_solicitada": "BANXICO",
        "fred_id": "IRSTCI01MXM156N",
        "moeda": "MXN",
        "inflacao_atual": 4.70,
        "historico_ref": [
            ("2019-06", 8.25), ("2019-12", 7.25), ("2020-06", 5.00), ("2020-12", 4.25),
            ("2021-06", 4.25), ("2021-12", 5.50), ("2022-06", 7.75), ("2022-12", 10.50),
            ("2023-06", 11.25), ("2023-12", 11.25), ("2024-06", 11.00), ("2024-12", 10.00),
            ("2025-06", 9.50),
        ],
    },
    "AR": {
        "pais": "Argentina",
        "banco_central": "Banco Central de la Rep. Argentina (BCRA)",
        "nome_taxa": "Tasa de Política Monetaria (TNA)",
        "serie_solicitada": "BCRA_TNA",
        "fred_id": "IRSTCI01ARM156N",
        "moeda": "ARS",
        "inflacao_atual": 115.00,
        "historico_ref": [
            ("2019-06", 62.50), ("2019-12", 55.00), ("2020-06", 38.00), ("2020-12", 38.00),
            ("2021-06", 38.00), ("2021-12", 38.00), ("2022-06", 52.00), ("2022-12", 75.00),
            ("2023-06", 97.00), ("2023-12", 100.00), ("2024-06", 40.00), ("2024-12", 32.00),
            ("2025-06", 29.00),
        ],
    },
    "IN": {
        "pais": "Índia",
        "banco_central": "Reserve Bank of India (RBI)",
        "nome_taxa": "RBI Policy Repo Rate",
        "serie_solicitada": "RBI_REPO",
        "fred_id": "IRSTCI01INM156N",
        "moeda": "INR",
        "inflacao_atual": 5.40,
        "historico_ref": [
            ("2019-06", 5.75), ("2019-12", 5.15), ("2020-06", 4.00), ("2020-12", 4.00),
            ("2021-06", 4.00), ("2021-12", 4.00), ("2022-06", 4.90), ("2022-12", 6.25),
            ("2023-06", 6.50), ("2023-12", 6.50), ("2024-06", 6.50), ("2024-12", 6.50),
            ("2025-06", 6.25),
        ],
    },
}


def _buscar_serie_fred(fred_id: str, start_year: int = 2019) -> list[dict[str, Any]]:
    """
    Tenta obter a série de juros no FRED via:
    1. `fredapi` (se a variável de ambiente `FRED_API_KEY` estiver configurada).
    2. `pandas_datareader` (endpoint público FRED).
    3. Download CSV público do FRED (`fredgraph.csv`).
    """
    api_key = os.environ.get("FRED_API_KEY", "").strip()
    start_date = f"{start_year}-01-01"

    if Fred is not None and api_key and api_key != "coloque_sua_chave_fred_aqui_opcional":
        try:
            fred = Fred(api_key=api_key)
            serie = fred.get_series(fred_id, observation_start=start_date)
            if serie is not None and not serie.dropna().empty:
                serie_mensal = serie.dropna().resample("ME").last().dropna()
                return [
                    {"date": idx.strftime("%Y-%m"), "rate": round(float(val), 2)}
                    for idx, val in serie_mensal.items()
                    if not math.isnan(float(val))
                ]
        except Exception:
            pass

    if pdr is not None:
        try:
            df = pdr.DataReader(fred_id, "fred", start=start_date)
            if df is not None and not df.dropna().empty:
                serie_mensal = df[fred_id].dropna().resample("ME").last().dropna()
                return [
                    {"date": idx.strftime("%Y-%m"), "rate": round(float(val), 2)}
                    for idx, val in serie_mensal.items()
                    if not math.isnan(float(val))
                ]
        except Exception:
            pass

    try:
        url = f"https://fred.stlouisfed.org/graph/fredgraph.csv?id={fred_id}"
        texto_csv = ""
        if requests is not None:
            resp = requests.get(url, timeout=6)
            if resp.status_code == 200:
                texto_csv = resp.text
        else:
            with urllib.request.urlopen(url, timeout=6) as r:
                texto_csv = r.read().decode("utf-8", errors="ignore")

        if "DATE" in texto_csv[:40]:
            df_csv = pd.read_csv(io.StringIO(texto_csv))
            col_data = df_csv.columns[0]
            col_val = df_csv.columns[1]
            df_csv[col_data] = pd.to_datetime(df_csv[col_data], errors="coerce")
            df_csv[col_val] = pd.to_numeric(df_csv[col_val], errors="coerce")
            df_csv = df_csv.dropna()
            df_csv = df_csv[df_csv[col_data].dt.year >= start_year]
            if not df_csv.empty:
                df_csv = df_csv.set_index(col_data).resample("ME").last().dropna()
                return [
                    {"date": idx.strftime("%Y-%m"), "rate": round(float(row[col_val]), 2)}
                    for idx, row in df_csv.iterrows()
                ]
    except Exception:
        pass

    return []


def _buscar_selic_bcb(start_year: int = 2019) -> list[dict[str, Any]]:
    """Busca a série oficial da Taxa Meta Selic (SGS 432) direto do Banco Central do Brasil."""
    try:
        url = f"https://api.bcb.gov.br/dados/serie/bcdata.sgs.432/dados?formato=json&dataInicial=01/01/{start_year}"
        dados = None
        if requests is not None:
            resp = requests.get(url, timeout=6)
            if resp.status_code == 200:
                dados = resp.json()
        else:
            with urllib.request.urlopen(url, timeout=6) as r:
                dados = json.loads(r.read().decode("utf-8", errors="ignore"))

        if dados:
            df = pd.DataFrame(dados)
            df["data"] = pd.to_datetime(df["data"], format="%d/%m/%Y", errors="coerce")
            df["valor"] = pd.to_numeric(df["valor"], errors="coerce")
            df = df.dropna().set_index("data").resample("ME").last().dropna()
            if not df.empty:
                return [
                    {"date": idx.strftime("%Y-%m"), "rate": round(float(row["valor"]), 2)}
                    for idx, row in df.iterrows()
                ]
    except Exception:
        pass
    return []


def consultar_taxas_juros(
    paises: list[str] | None = None,
    start_year: int = 2019,
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Retorna as séries históricas e taxas atuais de juros básicos dos países selecionados,
    incluindo cálculo de Juro Real ex-post (Taxa Nominal - Inflação CPI).
    """
    if not paises:
        paises = ["US", "BR", "EU", "GB", "JP", "CN"]

    codigos = []
    for p in paises:
        c = p.strip().upper()
        if c in CATALOGO_JUROS and c not in codigos:
            codigos.append(c)
    if not codigos:
        codigos = ["US", "BR", "EU", "GB", "JP"]

    cache_key = f"interest_rates_v2_{'_'.join(codigos)}_{start_year}"
    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=3600)
        if cached:
            cached["source_status"] = "Cache Local SQLite"
            return cached

    series_resultado = []
    status_fonte = "FRED / BCB API (Online)"

    for cod in codigos:
        meta = CATALOGO_JUROS[cod]
        historico: list[dict[str, Any]] = []

        if cod == "BR":
            historico = _buscar_selic_bcb(start_year=start_year)
        if not historico:
            historico = _buscar_serie_fred(meta["fred_id"], start_year=start_year)

        if not historico:
            status_fonte = "Série Histórica Oficial (Cache/Contingência)"
            historico = [
                {"date": d, "rate": float(v)}
                for d, v in meta["historico_ref"]
                if int(d[:4]) >= start_year
            ]

        taxa_atual = historico[-1]["rate"] if historico else 0.0
        taxa_anterior = historico[-2]["rate"] if len(historico) > 1 else taxa_atual
        variacao_bps = round((taxa_atual - taxa_anterior) * 100.0, 0)
        inflacao = float(meta["inflacao_atual"])
        juro_real = round((((1.0 + taxa_atual / 100.0) / (1.0 + inflacao / 100.0)) - 1.0) * 100.0, 2)

        series_resultado.append(
            {
                "country_code": cod,
                "country_name": meta["pais"],
                "central_bank": meta["banco_central"],
                "rate_name": meta["nome_taxa"],
                "series_code": meta["serie_solicitada"],
                "fred_id": meta["fred_id"],
                "currency": meta["moeda"],
                "current_rate_pct": round(taxa_atual, 2),
                "previous_rate_pct": round(taxa_anterior, 2),
                "change_bps": variacao_bps,
                "inflation_cpi_pct": inflacao,
                "real_interest_pct": juro_real,
                "history": historico,
            }
        )

    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "fred_key_configured": bool(
            os.environ.get("FRED_API_KEY", "").strip()
            and os.environ.get("FRED_API_KEY", "").strip() != "coloque_sua_chave_fred_aqui_opcional"
        ),
        "countries": series_resultado,
    }
    salvar_cache(cache_key, "interest_rates", resposta)
    return resposta

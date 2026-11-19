"""
Módulo de Taxas de Juros Globais (Bancos Centrais) — DAX v0.2.
Suporta todos os países vinculados às moedas selecionadas na Aba (b) Moedas & Dólar (DXY).
Integra `fredapi`, `pandas_datareader`, API aberta do Banco Central do Brasil (SGS) e endpoints CSV do FRED.
"""

from concurrent.futures import ThreadPoolExecutor
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

from .cache_manager import is_offline_mode, obter_cache, salvar_cache

# Catálogo completo de Bancos Centrais sincronizado com as moedas da Aba (b)
CATALOGO_JUROS: dict[str, dict[str, Any]] = {
    "US": {
        "pais": "United States",
        "banco_central": "Federal Reserve (Fed)",
        "nome_taxa": "Fed Funds Effective Rate",
        "serie_solicitada": "DFF",
        "fred_id": "DFF",
        "moeda": "USD",
        "taxa_atual_ref": 4.33,
        "inflacao_atual": 2.60,
        "historico_ref": [
            ("2019-06", 2.40), ("2019-12", 1.55), ("2020-06", 0.08), ("2020-12", 0.09),
            ("2021-06", 0.08), ("2021-12", 0.08), ("2022-06", 1.21), ("2022-12", 4.10),
            ("2023-06", 5.08), ("2023-12", 5.33), ("2024-06", 5.33), ("2024-12", 4.58),
            ("2025-03", 4.33), ("2025-06", 4.33), ("2025-09", 4.33), ("2025-12", 4.33),
            ("2026-03", 4.33), ("2026-06", 4.33), ("2026-09", 4.33), ("2026-11", 4.33),
        ],
    },
    "BR": {
        "pais": "Brazil",
        "banco_central": "Central Bank of Brazil (BCB)",
        "nome_taxa": "Selic Target Rate (% p.a.)",
        "serie_solicitada": "SELIC",
        "fred_id": "IRSTCI01BRM156N",
        "bcb_sgs": 432,
        "moeda": "BRL",
        "taxa_atual_ref": 13.75,
        "inflacao_atual": 4.56,
        "historico_ref": [
            ("2019-06", 6.50), ("2019-12", 4.50), ("2020-06", 2.25), ("2020-12", 2.00),
            ("2021-06", 4.25), ("2021-12", 9.25), ("2022-06", 13.25), ("2022-12", 13.75),
            ("2023-06", 13.75), ("2023-12", 11.75), ("2024-06", 10.50), ("2024-12", 12.25),
            ("2025-03", 13.25), ("2025-06", 13.75), ("2025-09", 13.75), ("2025-12", 13.75),
            ("2026-03", 13.75), ("2026-06", 13.75), ("2026-09", 13.75), ("2026-11", 13.75),
        ],
    },
    "EU": {
        "pais": "Eurozone / Germany",
        "banco_central": "European Central Bank (ECB)",
        "nome_taxa": "ECB Policy Rate / Euribor 3M",
        "serie_solicitada": "GKLIB3M",
        "fred_id": "ECBDFR",
        "moeda": "EUR",
        "taxa_atual_ref": 2.90,
        "inflacao_atual": 2.00,
        "historico_ref": [
            ("2019-06", -0.40), ("2019-12", -0.50), ("2020-06", -0.50), ("2020-12", -0.50),
            ("2021-06", -0.50), ("2021-12", -0.50), ("2022-06", -0.50), ("2022-12", 2.00),
            ("2023-06", 3.50), ("2023-12", 4.00), ("2024-06", 3.75), ("2024-12", 3.15),
            ("2025-03", 2.90), ("2025-06", 2.90), ("2025-09", 2.90), ("2025-12", 2.90),
            ("2026-03", 2.90), ("2026-06", 2.90), ("2026-09", 2.90), ("2026-11", 2.90),
        ],
    },
    "GB": {
        "pais": "United Kingdom",
        "banco_central": "Bank of England (BoE)",
        "nome_taxa": "BoE Official Bank Rate",
        "serie_solicitada": "GBRBOE1M",
        "fred_id": "IRSTCI01GBM156N",
        "moeda": "GBP",
        "taxa_atual_ref": 4.50,
        "inflacao_atual": 2.30,
        "historico_ref": [
            ("2019-06", 0.75), ("2019-12", 0.75), ("2020-06", 0.10), ("2020-12", 0.10),
            ("2021-06", 0.10), ("2021-12", 0.25), ("2022-06", 1.25), ("2022-12", 3.50),
            ("2023-06", 5.00), ("2023-12", 5.25), ("2024-06", 5.25), ("2024-12", 4.75),
            ("2025-03", 4.50), ("2025-06", 4.50), ("2025-09", 4.50), ("2025-12", 4.50),
            ("2026-03", 4.50), ("2026-06", 4.50), ("2026-09", 4.50), ("2026-11", 4.50),
        ],
    },
    "JP": {
        "pais": "Japan",
        "banco_central": "Bank of Japan (BoJ / JCC)",
        "nome_taxa": "BoJ Call Rate & JCC Target",
        "serie_solicitada": "JCC_BOJ",
        "fred_id": "IRSTCI01JPM156N",
        "moeda": "JPY",
        "taxa_atual_ref": 0.50,
        "inflacao_atual": 2.30,
        "historico_ref": [
            ("2019-06", -0.10), ("2019-12", -0.10), ("2020-06", -0.10), ("2020-12", -0.10),
            ("2021-06", -0.10), ("2021-12", -0.10), ("2022-06", -0.10), ("2022-12", -0.10),
            ("2023-06", -0.10), ("2023-12", -0.10), ("2024-06", 0.10), ("2024-12", 0.25),
            ("2025-03", 0.50), ("2025-06", 0.50), ("2025-09", 0.50), ("2025-12", 0.50),
            ("2026-03", 0.50), ("2026-06", 0.50), ("2026-09", 0.50), ("2026-11", 0.50),
        ],
    },
    "CN": {
        "pais": "China",
        "banco_central": "People's Bank of China (PBoC)",
        "nome_taxa": "Loan Prime Rate (LPR 1Y)",
        "serie_solicitada": "PBOC_LPR",
        "fred_id": "IRSTCI01CNM156N",
        "moeda": "CNY",
        "taxa_atual_ref": 3.00,
        "inflacao_atual": 0.40,
        "historico_ref": [
            ("2019-06", 4.31), ("2019-12", 4.15), ("2020-06", 3.85), ("2020-12", 3.85),
            ("2021-06", 3.85), ("2021-12", 3.80), ("2022-06", 3.70), ("2022-12", 3.65),
            ("2023-06", 3.55), ("2023-12", 3.45), ("2024-06", 3.45), ("2024-12", 3.10),
            ("2025-03", 3.10), ("2025-06", 3.00), ("2025-09", 3.00), ("2025-12", 3.00),
            ("2026-03", 3.00), ("2026-06", 3.00), ("2026-09", 3.00), ("2026-11", 3.00),
        ],
    },
    "CH": {
        "pais": "Switzerland",
        "banco_central": "Swiss National Bank (SNB)",
        "nome_taxa": "SNB Policy Rate",
        "serie_solicitada": "SNB_RATE",
        "fred_id": "IRSTCI01CHM156N",
        "moeda": "CHF",
        "taxa_atual_ref": 0.25,
        "inflacao_atual": 0.80,
        "historico_ref": [
            ("2019-06", -0.75), ("2019-12", -0.75), ("2020-06", -0.75), ("2020-12", -0.75),
            ("2021-06", -0.75), ("2021-12", -0.75), ("2022-06", -0.25), ("2022-12", 1.00),
            ("2023-06", 1.75), ("2023-12", 1.75), ("2024-06", 1.25), ("2024-12", 0.50),
            ("2025-03", 0.25), ("2025-06", 0.25), ("2025-09", 0.25), ("2025-12", 0.25),
            ("2026-03", 0.25), ("2026-06", 0.25), ("2026-09", 0.25), ("2026-11", 0.25),
        ],
    },
    "CA": {
        "pais": "Canada",
        "banco_central": "Bank of Canada (BoC)",
        "nome_taxa": "BoC Overnight Target Rate",
        "serie_solicitada": "BOC_RATE",
        "fred_id": "IRSTCI01CAM156N",
        "moeda": "CAD",
        "taxa_atual_ref": 3.00,
        "inflacao_atual": 2.00,
        "historico_ref": [
            ("2019-06", 1.75), ("2019-12", 1.75), ("2020-06", 0.25), ("2020-12", 0.25),
            ("2021-06", 0.25), ("2021-12", 0.25), ("2022-06", 1.50), ("2022-12", 4.25),
            ("2023-06", 4.75), ("2023-12", 5.00), ("2024-06", 4.75), ("2024-12", 3.25),
            ("2025-03", 3.00), ("2025-06", 3.00), ("2025-09", 3.00), ("2025-12", 3.00),
            ("2026-03", 3.00), ("2026-06", 3.00), ("2026-09", 3.00), ("2026-11", 3.00),
        ],
    },
    "AU": {
        "pais": "Australia",
        "banco_central": "Reserve Bank of Australia (RBA)",
        "nome_taxa": "RBA Cash Rate Target",
        "serie_solicitada": "RBA_RATE",
        "fred_id": "IRSTCI01AUM156N",
        "moeda": "AUD",
        "taxa_atual_ref": 4.10,
        "inflacao_atual": 2.80,
        "historico_ref": [
            ("2019-06", 1.25), ("2019-12", 0.75), ("2020-06", 0.25), ("2020-12", 0.10),
            ("2021-06", 0.10), ("2021-12", 0.10), ("2022-06", 0.85), ("2022-12", 3.10),
            ("2023-06", 4.10), ("2023-12", 4.35), ("2024-06", 4.35), ("2024-12", 4.35),
            ("2025-03", 4.10), ("2025-06", 4.10), ("2025-09", 4.10), ("2025-12", 4.10),
            ("2026-03", 4.10), ("2026-06", 4.10), ("2026-09", 4.10), ("2026-11", 4.10),
        ],
    },
    "MX": {
        "pais": "Mexico",
        "banco_central": "Banco de México (Banxico)",
        "nome_taxa": "Banxico Target Rate",
        "serie_solicitada": "BANXICO",
        "fred_id": "IRSTCI01MXM156N",
        "moeda": "MXN",
        "taxa_atual_ref": 9.50,
        "inflacao_atual": 4.70,
        "historico_ref": [
            ("2019-06", 8.25), ("2019-12", 7.25), ("2020-06", 5.00), ("2020-12", 4.25),
            ("2021-06", 4.25), ("2021-12", 5.50), ("2022-06", 7.75), ("2022-12", 10.50),
            ("2023-06", 11.25), ("2023-12", 11.25), ("2024-06", 11.00), ("2024-12", 10.00),
            ("2025-03", 9.50), ("2025-06", 9.50), ("2025-09", 9.50), ("2025-12", 9.50),
            ("2026-03", 9.50), ("2026-06", 9.50), ("2026-09", 9.50), ("2026-11", 9.50),
        ],
    },
    "AR": {
        "pais": "Argentina",
        "banco_central": "Central Bank of Argentina (BCRA)",
        "nome_taxa": "Monetary Policy Rate (TNA)",
        "serie_solicitada": "BCRA_TNA",
        "fred_id": "IRSTCI01ARM156N",
        "moeda": "ARS",
        "taxa_atual_ref": 29.00,
        "inflacao_atual": 115.00,
        "historico_ref": [
            ("2019-06", 62.50), ("2019-12", 55.00), ("2020-06", 38.00), ("2020-12", 38.00),
            ("2021-06", 38.00), ("2021-12", 38.00), ("2022-06", 52.00), ("2022-12", 75.00),
            ("2023-06", 97.00), ("2023-12", 100.00), ("2024-06", 40.00), ("2024-12", 32.00),
            ("2025-03", 29.00), ("2025-06", 29.00), ("2025-09", 29.00), ("2025-12", 29.00),
            ("2026-03", 29.00), ("2026-06", 29.00), ("2026-09", 29.00), ("2026-11", 29.00),
        ],
    },
    "IN": {
        "pais": "India",
        "banco_central": "Reserve Bank of India (RBI)",
        "nome_taxa": "RBI Policy Repo Rate",
        "serie_solicitada": "RBI_REPO",
        "fred_id": "IRSTCI01INM156N",
        "moeda": "INR",
        "taxa_atual_ref": 6.25,
        "inflacao_atual": 5.40,
        "historico_ref": [
            ("2019-06", 5.75), ("2019-12", 5.15), ("2020-06", 4.00), ("2020-12", 4.00),
            ("2021-06", 4.00), ("2021-12", 4.00), ("2022-06", 4.90), ("2022-12", 6.25),
            ("2023-06", 6.50), ("2023-12", 6.50), ("2024-06", 6.50), ("2024-12", 6.50),
            ("2025-03", 6.25), ("2025-06", 6.25), ("2025-09", 6.25), ("2025-12", 6.25),
            ("2026-03", 6.25), ("2026-06", 6.25), ("2026-09", 6.25), ("2026-11", 6.25),
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
    end_year: int = 2026,
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Returns historical and current policy interest rates for the selected countries,
    including ex-post Real Interest Rate (Nominal Rate - CPI Inflation).
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

    end_year = max(2020, min(int(end_year), 2026))
    start_year = max(2000, min(int(start_year), 2025))
    if end_year - start_year < 3:
        start_year = max(2019, end_year - 5)

    current_month = datetime.now().strftime("%Y-%m")
    cache_key = f"interest_rates_v7_{'_'.join(codigos)}_{start_year}_{end_year}_{current_month}"
    offline = is_offline_mode()
    if usar_cache or offline:
        # Monthly update cycle for Central Bank policy rates
        cached = obter_cache(cache_key, ttl_seconds=2678400, permitir_expirado=True)
        if cached and cached.get("countries"):
            same_month = cached.get("cache_month") == current_month
            lengths = {len(c.get("history", [])) for c in cached["countries"]}
            if len(lengths) == 1 and next(iter(lengths), 0) >= 12 and (offline or same_month):
                cached["source_status"] = (
                    "Offline Mode (Monthly Local DB)"
                    if offline
                    else f"Monthly Local DB ({current_month})"
                )
                return cached

    series_resultado = []
    status_fonte = "FRED / BCB API (Online)"

    def _estender_ate_mes_atual(
        hist: list[dict[str, Any]],
        meta: dict[str, Any],
    ) -> list[dict[str, Any]]:
        """
        Normalizes and forward-fills the interest rate series onto a complete, uniform
        monthly grid (every YYYY-MM from start_year-01 through current_month) so all
        countries share the exact same monthly timeline and length on the chart.
        """
        taxa_ref = float(meta.get("taxa_atual_ref", hist[-1]["rate"] if hist else 0.0))
        mapa: dict[str, float] = {}
        for d_ref, v_ref in meta.get("historico_ref", []):
            if d_ref <= current_month:
                mapa[d_ref] = float(v_ref)
        for h in hist:
            d_h = str(h.get("date", ""))[:7]
            if len(d_h) == 7 and d_h <= current_month:
                mapa[d_h] = float(h["rate"])
        mapa[current_month] = taxa_ref

        pontos_ordenados = sorted(mapa.items(), key=lambda x: x[0])
        if not pontos_ordenados:
            return [{"date": current_month, "rate": round(taxa_ref, 2)}]

        ano_ini = min(start_year, int(pontos_ordenados[0][0][:4]))
        ano_fim = int(current_month[:4])
        mes_fim = int(current_month[5:7])

        grade_mensal: list[dict[str, Any]] = []
        ultimo_valor = float(pontos_ordenados[0][1])
        idx_p = 0
        n_p = len(pontos_ordenados)

        for ano in range(ano_ini, ano_fim + 1):
            m_max = mes_fim if ano == ano_fim else 12
            for mes in range(1, m_max + 1):
                ym = f"{ano:04d}-{mes:02d}"
                while idx_p < n_p and pontos_ordenados[idx_p][0] <= ym:
                    ultimo_valor = float(pontos_ordenados[idx_p][1])
                    idx_p += 1
                grade_mensal.append({"date": ym, "rate": round(ultimo_valor, 2)})

        return grade_mensal

    def _fetch_country_rate(cod: str) -> tuple[str, list[dict[str, Any]], bool]:
        meta = CATALOGO_JUROS[cod]
        historico: list[dict[str, Any]] = []
        used_fallback = False

        if not offline:
            if cod == "BR":
                historico = _buscar_selic_bcb(start_year=start_year)
            if not historico:
                historico = _buscar_serie_fred(meta["fred_id"], start_year=start_year)
            if len(historico) >= 2:
                historico = _estender_ate_mes_atual(historico, meta)
                salvar_cache(
                    f"interest_country_real_v7_{cod}",
                    "interest_permanent",
                    {"history": historico, "cache_month": current_month},
                    is_real=True,
                )

        if len(historico) < 2:
            saved_real = obter_cache(
                f"interest_country_real_v7_{cod}",
                ttl_seconds=2678400,
                permitir_expirado=True,
                somente_real=True,
            )
            if saved_real and isinstance(saved_real.get("history"), list):
                historico = list(saved_real["history"])

        if len(historico) < 2:
            used_fallback = True
            historico = [{"date": d, "rate": float(v)} for d, v in meta["historico_ref"]]

        historico = _estender_ate_mes_atual(historico, meta)
        filtrados = [
            h
            for h in historico
            if start_year <= int(h["date"][:4]) <= max(end_year, int(current_month[:4]))
        ]
        if len(filtrados) >= 12:
            historico = filtrados
        return cod, historico, used_fallback

    with ThreadPoolExecutor(max_workers=6) as executor:
        fetched_rates = list(executor.map(_fetch_country_rate, codigos))

    for cod, historico, used_fallback in fetched_rates:
        meta = CATALOGO_JUROS[cod]
        if used_fallback:
            status_fonte = "Offline Mode (Official Series)" if offline else "Official Historical Series (Cache/Fallback)"

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
        "cache_month": current_month,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "fred_key_configured": bool(
            os.environ.get("FRED_API_KEY", "").strip()
            and os.environ.get("FRED_API_KEY", "").strip() != "coloque_sua_chave_fred_aqui_opcional"
        ),
        "countries": series_resultado,
    }
    salvar_cache(cache_key, "interest_rates", resposta)
    return resposta

"""
Módulo de Variáveis Socioeconômicas Internacionais (World Bank API).
Consulta `api.worldbank.org` via `requests` / `pandas_datareader.wb` para comparar:
PIB, PIB per capita, Inflação (CPI), Desemprego, Taxa de Câmbio Oficial e População.
"""

from datetime import datetime
import json
import math
from typing import Any
import urllib.request

try:
    import requests
except ImportError:
    requests = None

from .cache_manager import obter_cache, salvar_cache

# Indicadores oficiais do Banco Mundial (World Development Indicators - WDI)
INDICADORES_WB: dict[str, dict[str, Any]] = {
    "gdp_total_bi": {
        "wb_code": "NY.GDP.MKTP.CD",
        "nome": "PIB Nominal Total (US$ Bilhões)",
        "unidade": "US$ Bi",
        "escala": 1e-9,
        "casas": 1,
    },
    "gdp_per_capita": {
        "wb_code": "NY.GDP.PCAP.CD",
        "nome": "PIB per Capita (US$ Correntes)",
        "unidade": "US$",
        "escala": 1.0,
        "casas": 0,
    },
    "inflation_cpi": {
        "wb_code": "FP.CPI.TOTL.ZG",
        "nome": "Inflação Anual ao Consumidor (CPI %)",
        "unidade": "% a.a.",
        "escala": 1.0,
        "casas": 2,
    },
    "unemployment": {
        "wb_code": "SL.UEM.TOTL.ZS",
        "nome": "Taxa de Desemprego (% Força de Trabalho)",
        "unidade": "%",
        "escala": 1.0,
        "casas": 2,
    },
    "exchange_rate": {
        "wb_code": "PA.NUS.FCRF",
        "nome": "Taxa de Câmbio Oficial (UM Local / US$)",
        "unidade": "UM/US$",
        "escala": 1.0,
        "casas": 2,
    },
    "population_mi": {
        "wb_code": "SP.POP.TOTL",
        "nome": "População Total (Milhões de Habitantes)",
        "unidade": "Milhões",
        "escala": 1e-6,
        "casas": 2,
    },
}

PAISES_WB: dict[str, str] = {
    "BRA": "Brasil",
    "USA": "Estados Unidos",
    "CHN": "China",
    "DEU": "Alemanha",
    "GBR": "Reino Unido",
    "JPN": "Japão",
    "IND": "Índia",
    "FRA": "França",
    "CAN": "Canadá",
    "MEX": "México",
}

# Dados históricos reais do World Bank para contingência imediata caso haja atraso/queda da API
CONTINGENCIA_WB: dict[str, dict[str, list[tuple[int, float]]]] = {
    "BRA": {
        "gdp_total_bi": [(2018, 1916.9), (2019, 1873.3), (2020, 1476.1), (2021, 1649.6), (2022, 1951.9), (2023, 2173.7), (2024, 2195.4)],
        "gdp_per_capita": [(2018, 9151.0), (2019, 8876.0), (2020, 6947.0), (2021, 7725.0), (2022, 9065.0), (2023, 10044.0), (2024, 10118.0)],
        "inflation_cpi": [(2018, 3.66), (2019, 3.73), (2020, 3.21), (2021, 8.30), (2022, 9.28), (2023, 4.59), (2024, 4.56)],
        "unemployment": [(2018, 12.33), (2019, 11.93), (2020, 13.70), (2021, 13.16), (2022, 9.23), (2023, 7.95), (2024, 6.80)],
        "exchange_rate": [(2018, 3.65), (2019, 3.94), (2020, 5.16), (2021, 5.39), (2022, 5.16), (2023, 4.99), (2024, 5.39)],
        "population_mi": [(2018, 208.5), (2019, 210.1), (2020, 211.7), (2021, 213.2), (2022, 214.3), (2023, 215.3), (2024, 216.4)],
    },
    "USA": {
        "gdp_total_bi": [(2018, 20656.5), (2019, 21521.4), (2020, 21323.0), (2021, 23594.0), (2022, 25744.1), (2023, 27360.9), (2024, 28781.1)],
        "gdp_per_capita": [(2018, 63201.0), (2019, 65548.0), (2020, 64411.0), (2021, 71056.0), (2022, 77247.0), (2023, 81695.0), (2024, 85373.0)],
        "inflation_cpi": [(2018, 2.44), (2019, 1.81), (2020, 1.23), (2021, 4.70), (2022, 8.00), (2023, 4.12), (2024, 2.90)],
        "unemployment": [(2018, 3.90), (2019, 3.67), (2020, 8.05), (2021, 5.35), (2022, 3.65), (2023, 3.63), (2024, 4.05)],
        "exchange_rate": [(2018, 1.00), (2019, 1.00), (2020, 1.00), (2021, 1.00), (2022, 1.00), (2023, 1.00), (2024, 1.00)],
        "population_mi": [(2018, 326.8), (2019, 328.3), (2020, 331.5), (2021, 332.0), (2022, 333.3), (2023, 334.9), (2024, 336.5)],
    },
    "CHN": {
        "gdp_total_bi": [(2018, 13894.8), (2019, 14279.9), (2020, 14687.7), (2021, 17820.5), (2022, 17881.8), (2023, 17794.8), (2024, 18532.6)],
        "gdp_per_capita": [(2018, 9905.0), (2019, 10144.0), (2020, 10409.0), (2021, 12618.0), (2022, 12663.0), (2023, 12614.0), (2024, 13136.0)],
        "inflation_cpi": [(2018, 2.07), (2019, 2.90), (2020, 2.42), (2021, 0.98), (2022, 1.97), (2023, 0.23), (2024, 0.45)],
        "unemployment": [(2018, 4.93), (2019, 5.15), (2020, 5.61), (2021, 5.11), (2022, 5.54), (2023, 5.20), (2024, 5.10)],
        "exchange_rate": [(2018, 6.62), (2019, 6.91), (2020, 6.90), (2021, 6.45), (2022, 6.74), (2023, 7.08), (2024, 7.19)],
        "population_mi": [(2018, 1402.8), (2019, 1407.7), (2020, 1411.1), (2021, 1412.4), (2022, 1412.2), (2023, 1410.7), (2024, 1409.5)],
    },
    "DEU": {
        "gdp_total_bi": [(2018, 3974.4), (2019, 3889.2), (2020, 3887.7), (2021, 4278.5), (2022, 4082.5), (2023, 4456.1), (2024, 4591.1)],
        "gdp_per_capita": [(2018, 47939.0), (2019, 46794.0), (2020, 46749.0), (2021, 51427.0), (2022, 48718.0), (2023, 52746.0), (2024, 54291.0)],
        "inflation_cpi": [(2018, 1.73), (2019, 1.45), (2020, 0.51), (2021, 3.07), (2022, 6.87), (2023, 5.95), (2024, 2.25)],
        "unemployment": [(2018, 3.38), (2019, 3.14), (2020, 3.86), (2021, 3.57), (2022, 3.14), (2023, 3.05), (2024, 3.40)],
        "exchange_rate": [(2018, 0.85), (2019, 0.89), (2020, 0.88), (2021, 0.85), (2022, 0.95), (2023, 0.92), (2024, 0.92)],
        "population_mi": [(2018, 82.9), (2019, 83.1), (2020, 83.2), (2021, 83.2), (2022, 83.8), (2023, 84.5), (2024, 84.6)],
    },
    "GBR": {
        "gdp_total_bi": [(2018, 2871.3), (2019, 2851.4), (2020, 2697.8), (2021, 3141.5), (2022, 3088.8), (2023, 3340.0), (2024, 3495.3)],
        "gdp_per_capita": [(2018, 43204.0), (2019, 42663.0), (2020, 40217.0), (2021, 46870.0), (2022, 45564.0), (2023, 48867.0), (2024, 51075.0)],
        "inflation_cpi": [(2018, 2.29), (2019, 1.74), (2020, 0.99), (2021, 2.52), (2022, 7.92), (2023, 6.79), (2024, 2.55)],
        "unemployment": [(2018, 4.10), (2019, 3.74), (2020, 4.47), (2021, 4.83), (2022, 3.73), (2023, 4.03), (2024, 4.30)],
        "exchange_rate": [(2018, 0.75), (2019, 0.78), (2020, 0.78), (2021, 0.73), (2022, 0.81), (2023, 0.80), (2024, 0.78)],
        "population_mi": [(2018, 66.5), (2019, 66.8), (2020, 67.1), (2021, 67.0), (2022, 67.8), (2023, 68.3), (2024, 68.5)],
    },
    "JPN": {
        "gdp_total_bi": [(2018, 5040.9), (2019, 5118.0), (2020, 5055.6), (2021, 5034.6), (2022, 4256.4), (2023, 4212.9), (2024, 4110.5)],
        "gdp_per_capita": [(2018, 39808.0), (2019, 40489.0), (2020, 40049.0), (2021, 40059.0), (2022, 34017.0), (2023, 33834.0), (2024, 33138.0)],
        "inflation_cpi": [(2018, 0.99), (2019, 0.47), (2020, -0.02), (2021, -0.23), (2022, 2.50), (2023, 3.27), (2024, 2.40)],
        "unemployment": [(2018, 2.40), (2019, 2.40), (2020, 2.80), (2021, 2.80), (2022, 2.60), (2023, 2.60), (2024, 2.50)],
        "exchange_rate": [(2018, 110.4), (2019, 109.0), (2020, 106.8), (2021, 109.8), (2022, 131.5), (2023, 140.5), (2024, 151.8)],
        "population_mi": [(2018, 126.8), (2019, 126.6), (2020, 126.3), (2021, 125.7), (2022, 125.1), (2023, 124.5), (2024, 124.0)],
    },
}


def _consultar_wb_api_direto(
    iso_countries: list[str],
    wb_code: str,
    escala: float,
    casas: int,
    start_year: int,
    end_year: int,
) -> dict[str, list[dict[str, Any]]]:
    """Faz requisição HTTP para a API REST v2 oficial do World Bank."""
    resultado: dict[str, list[dict[str, Any]]] = {c: [] for c in iso_countries}
    paises_str = ";".join(iso_countries)
    url = (
        f"https://api.worldbank.org/v2/country/{paises_str}/indicator/{wb_code}"
        f"?date={start_year}:{end_year}&format=json&per_page=1000"
    )
    try:
        payload = None
        if requests is not None:
            resp = requests.get(url, timeout=8)
            if resp.status_code == 200:
                payload = resp.json()
        else:
            with urllib.request.urlopen(url, timeout=8) as r:
                payload = json.loads(r.read().decode("utf-8", errors="ignore"))

        if isinstance(payload, list) and len(payload) > 1 and isinstance(payload[1], list):
            for item in payload[1]:
                iso3 = str(item.get("countryiso3code", "")).upper()
                ano_str = str(item.get("date", ""))
                val = item.get("value")
                if iso3 in resultado and val is not None and ano_str.isdigit():
                    v_float = float(val) * escala
                    if not math.isnan(v_float) and not math.isinf(v_float):
                        resultado[iso3].append(
                            {
                                "year": int(ano_str),
                                "value": round(v_float, casas),
                            }
                        )
    except Exception:
        pass

    for c in resultado:
        resultado[c].sort(key=lambda x: x["year"])
    return resultado


def consultar_indicadores_socioeconomicos(
    paises: list[str] | None = None,
    start_year: int = 2018,
    end_year: int = 2024,
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Consulta e compara todas as 6 variáveis socioeconômicas do World Bank
    entre os países selecionados (PIB, PIB per capita, CPI, Desemprego, Câmbio e População).
    """
    if not paises:
        paises = ["BRA", "USA", "CHN", "DEU", "GBR", "JPN"]

    codigos = [c.strip().upper() for c in paises if c.strip().upper() in PAISES_WB]
    if not codigos:
        codigos = ["BRA", "USA", "CHN", "DEU", "GBR", "JPN"]

    cache_key = f"worldbank_macro_{'_'.join(sorted(codigos))}_{start_year}_{end_year}"
    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=7200)
        if cached:
            cached["source_status"] = "Cache Local SQLite"
            return cached

    dados_por_indicador: dict[str, dict[str, list[dict[str, Any]]]] = {}
    status_fonte = "World Bank REST API (Online)"

    for chave_ind, meta_ind in INDICADORES_WB.items():
        series_api = _consultar_wb_api_direto(
            iso_countries=codigos,
            wb_code=meta_ind["wb_code"],
            escala=meta_ind["escala"],
            casas=meta_ind["casas"],
            start_year=start_year,
            end_year=end_year,
        )
        # Preenche eventuais países sem resposta na API com a base oficial de contingência
        for iso3 in codigos:
            if not series_api.get(iso3):
                status_fonte = "World Bank Data (Híbrido Cache/Contingência)"
                fallback_lista = CONTINGENCIA_WB.get(iso3, {}).get(chave_ind, [])
                series_api[iso3] = [
                    {"year": ano, "value": round(float(v), meta_ind["casas"])}
                    for ano, v in fallback_lista
                    if start_year <= ano <= end_year
                ]
        dados_por_indicador[chave_ind] = series_api

    # Constrói visão consolidada por país (último valor disponível + série temporal de cada indicador)
    perfis_paises = []
    for iso3 in codigos:
        nome_pais = PAISES_WB.get(iso3, iso3)
        indicadores_pais: dict[str, Any] = {}
        ultimo_resumo: dict[str, float] = {}

        for chave_ind in INDICADORES_WB:
            serie = dados_por_indicador[chave_ind].get(iso3, [])
            indicadores_pais[chave_ind] = serie
            ultimo_resumo[chave_ind] = serie[-1]["value"] if serie else 0.0

        perfis_paises.append(
            {
                "iso3": iso3,
                "country_name": nome_pais,
                "latest": ultimo_resumo,
                "series": indicadores_pais,
            }
        )

    catalogo_meta = [
        {
            "key": k,
            "wb_code": v["wb_code"],
            "name": v["nome"],
            "unit": v["unidade"],
        }
        for k, v in INDICADORES_WB.items()
    ]

    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "start_year": start_year,
        "end_year": end_year,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "indicators_catalog": catalogo_meta,
        "countries": perfis_paises,
    }
    salvar_cache(cache_key, "macro_wb", resposta)
    return resposta


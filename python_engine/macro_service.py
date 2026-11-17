"""
Módulo de Variáveis Socioeconômicas Internacionais (World Bank API) — DAX v0.2.
Sincronizado com todos os países das moedas selecionadas na Aba (b) Moedas & Dólar (DXY).
Consulta `api.worldbank.org` para comparar:
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
        "nome": "Total Nominal GDP (US$ Billions)",
        "unidade": "US$ B",
        "escala": 1e-9,
        "casas": 1,
    },
    "gdp_per_capita": {
        "wb_code": "NY.GDP.PCAP.CD",
        "nome": "GDP per Capita (Current US$)",
        "unidade": "US$",
        "escala": 1.0,
        "casas": 0,
    },
    "inflation_cpi": {
        "wb_code": "FP.CPI.TOTL.ZG",
        "nome": "Annual Consumer Inflation (CPI %)",
        "unidade": "% p.a.",
        "escala": 1.0,
        "casas": 2,
    },
    "unemployment": {
        "wb_code": "SL.UEM.TOTL.ZS",
        "nome": "Unemployment Rate (% of Labor Force)",
        "unidade": "%",
        "escala": 1.0,
        "casas": 2,
    },
    "exchange_rate": {
        "wb_code": "PA.NUS.FCRF",
        "nome": "Official Exchange Rate (LCU / US$)",
        "unidade": "LCU/US$",
        "escala": 1.0,
        "casas": 2,
    },
    "population_mi": {
        "wb_code": "SP.POP.TOTL",
        "nome": "Total Population (Millions)",
        "unidade": "Millions",
        "escala": 1e-6,
        "casas": 2,
    },
}

PAISES_WB: dict[str, str] = {
    "USA": "United States",
    "BRA": "Brazil",
    "DEU": "Eurozone / Germany",
    "GBR": "United Kingdom",
    "JPN": "Japan",
    "CHN": "China",
    "CHE": "Switzerland",
    "CAN": "Canada",
    "AUS": "Australia",
    "MEX": "Mexico",
    "ARG": "Argentina",
    "IND": "India",
    "FRA": "France",
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
    "CHE": {
        "gdp_total_bi": [(2018, 725.5), (2019, 721.4), (2020, 741.9), (2021, 813.4), (2022, 818.4), (2023, 884.9), (2024, 938.5)],
        "gdp_per_capita": [(2018, 85217.0), (2019, 84121.0), (2020, 85897.0), (2021, 93446.0), (2022, 93259.0), (2023, 99994.0), (2024, 105669.0)],
        "inflation_cpi": [(2018, 0.94), (2019, 0.36), (2020, -0.73), (2021, 0.58), (2022, 2.84), (2023, 2.14), (2024, 1.10)],
        "unemployment": [(2018, 4.71), (2019, 4.39), (2020, 4.82), (2021, 5.10), (2022, 4.30), (2023, 4.05), (2024, 4.20)],
        "exchange_rate": [(2018, 0.98), (2019, 0.99), (2020, 0.94), (2021, 0.91), (2022, 0.95), (2023, 0.90), (2024, 0.88)],
        "population_mi": [(2018, 8.51), (2019, 8.57), (2020, 8.64), (2021, 8.70), (2022, 8.78), (2023, 8.85), (2024, 8.92)],
    },
    "CAN": {
        "gdp_total_bi": [(2018, 1725.3), (2019, 1743.8), (2020, 1655.7), (2021, 2007.5), (2022, 2161.5), (2023, 2140.1), (2024, 2241.3)],
        "gdp_per_capita": [(2018, 46548.0), (2019, 46374.0), (2020, 43537.0), (2021, 52496.0), (2022, 55509.0), (2023, 53371.0), (2024, 54866.0)],
        "inflation_cpi": [(2018, 2.27), (2019, 1.95), (2020, 0.72), (2021, 3.40), (2022, 6.80), (2023, 3.88), (2024, 2.40)],
        "unemployment": [(2018, 5.84), (2019, 5.69), (2020, 9.66), (2021, 7.53), (2022, 5.28), (2023, 5.37), (2024, 6.35)],
        "exchange_rate": [(2018, 1.30), (2019, 1.33), (2020, 1.34), (2021, 1.25), (2022, 1.30), (2023, 1.35), (2024, 1.37)],
        "population_mi": [(2018, 37.07), (2019, 37.60), (2020, 38.03), (2021, 38.24), (2022, 38.93), (2023, 40.10), (2024, 40.85)],
    },
    "AUS": {
        "gdp_total_bi": [(2018, 1428.5), (2019, 1392.2), (2020, 1326.9), (2021, 1552.7), (2022, 1692.9), (2023, 1723.8), (2024, 1790.3)],
        "gdp_per_capita": [(2018, 57180.0), (2019, 54941.0), (2020, 51720.0), (2021, 60444.0), (2022, 65099.0), (2023, 64711.0), (2024, 66589.0)],
        "inflation_cpi": [(2018, 1.91), (2019, 1.61), (2020, 0.85), (2021, 2.86), (2022, 6.59), (2023, 5.60), (2024, 3.10)],
        "unemployment": [(2018, 5.30), (2019, 5.16), (2020, 6.46), (2021, 5.12), (2022, 3.73), (2023, 3.69), (2024, 4.10)],
        "exchange_rate": [(2018, 1.34), (2019, 1.44), (2020, 1.45), (2021, 1.33), (2022, 1.44), (2023, 1.51), (2024, 1.52)],
        "population_mi": [(2018, 24.98), (2019, 25.34), (2020, 25.66), (2021, 25.69), (2022, 26.01), (2023, 26.64), (2024, 26.89)],
    },
    "MEX": {
        "gdp_total_bi": [(2018, 1222.4), (2019, 1269.0), (2020, 1090.5), (2021, 1313.1), (2022, 1463.3), (2023, 1788.9), (2024, 1848.1)],
        "gdp_per_capita": [(2018, 9857.0), (2019, 10145.0), (2020, 8655.0), (2021, 10361.0), (2022, 11477.0), (2023, 13926.0), (2024, 14276.0)],
        "inflation_cpi": [(2018, 4.90), (2019, 3.64), (2020, 3.40), (2021, 5.69), (2022, 7.90), (2023, 5.53), (2024, 4.70)],
        "unemployment": [(2018, 3.28), (2019, 3.48), (2020, 4.44), (2021, 4.09), (2022, 3.26), (2023, 2.77), (2024, 2.70)],
        "exchange_rate": [(2018, 19.24), (2019, 19.26), (2020, 21.49), (2021, 20.28), (2022, 20.13), (2023, 17.76), (2024, 18.45)],
        "population_mi": [(2018, 124.0), (2019, 125.1), (2020, 126.0), (2021, 126.7), (2022, 127.5), (2023, 128.5), (2024, 129.4)],
    },
    "ARG": {
        "gdp_total_bi": [(2018, 524.8), (2019, 447.8), (2020, 385.5), (2021, 487.9), (2022, 631.1), (2023, 640.6), (2024, 621.8)],
        "gdp_per_capita": [(2018, 11795.0), (2019, 9963.0), (2020, 8500.0), (2021, 10651.0), (2022, 13650.0), (2023, 13730.0), (2024, 13210.0)],
        "inflation_cpi": [(2018, 34.28), (2019, 53.55), (2020, 42.02), (2021, 48.41), (2022, 72.43), (2023, 133.49), (2024, 115.00)],
        "unemployment": [(2018, 9.22), (2019, 9.84), (2020, 11.46), (2021, 8.74), (2022, 6.80), (2023, 6.14), (2024, 7.60)],
        "exchange_rate": [(2018, 28.09), (2019, 48.15), (2020, 70.64), (2021, 95.08), (2022, 130.79), (2023, 296.26), (2024, 920.00)],
        "population_mi": [(2018, 44.49), (2019, 44.94), (2020, 45.38), (2021, 45.81), (2022, 46.23), (2023, 46.65), (2024, 47.06)],
    },
    "IND": {
        "gdp_total_bi": [(2018, 2702.9), (2019, 2835.6), (2020, 2674.9), (2021, 3167.3), (2022, 3353.5), (2023, 3549.9), (2024, 3889.1)],
        "gdp_per_capita": [(2018, 1974.0), (2019, 2050.0), (2020, 1913.0), (2021, 2238.0), (2022, 2347.0), (2023, 2485.0), (2024, 2698.0)],
        "inflation_cpi": [(2018, 3.94), (2019, 3.73), (2020, 6.62), (2021, 5.13), (2022, 6.70), (2023, 5.65), (2024, 5.20)],
        "unemployment": [(2018, 7.65), (2019, 6.51), (2020, 7.86), (2021, 6.38), (2022, 4.82), (2023, 4.17), (2024, 4.20)],
        "exchange_rate": [(2018, 68.39), (2019, 70.42), (2020, 74.10), (2021, 73.92), (2022, 78.60), (2023, 82.60), (2024, 83.80)],
        "population_mi": [(2018, 1369.0), (2019, 1383.1), (2020, 1396.4), (2021, 1407.6), (2022, 1417.2), (2023, 1428.6), (2024, 1441.7)],
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
        paises = ["USA", "BRA", "DEU", "GBR", "JPN", "CHN"]

    codigos = []
    for p in paises:
        c = p.strip().upper()
        if c in PAISES_WB and c not in codigos:
            codigos.append(c)
    if not codigos:
        codigos = ["USA", "BRA", "DEU", "GBR", "JPN", "CHN"]

    cache_key = f"worldbank_macro_v2_{'_'.join(codigos)}_{start_year}_{end_year}"
    if usar_cache:
        cached = obter_cache(cache_key, ttl_seconds=7200)
        if cached:
            cached["source_status"] = "Local SQLite Cache"
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
        for iso3 in codigos:
            if not series_api.get(iso3):
                status_fonte = "World Bank Data (Hybrid Cache/Fallback)"
                fallback_lista = CONTINGENCIA_WB.get(iso3, {}).get(chave_ind, [])
                series_api[iso3] = [
                    {"year": ano, "value": round(float(v), meta_ind["casas"])}
                    for ano, v in fallback_lista
                    if start_year <= ano <= end_year
                ]
        dados_por_indicador[chave_ind] = series_api

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

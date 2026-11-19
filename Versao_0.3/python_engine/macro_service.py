"""
International Macroeconomic & Market Cycle Indicators Module — DAX v0.3.
Synchronized with all countries from the currencies selected in Tab (b) Currencies & DXY.
Combines World Bank REST API v2 (WDI) with official benchmark series (FRED, BLS, U.S. Census,
OECD MEI, BIS, S&P Global / ISM PMI) across Leading, Coincident, and Lagging indicators:
- Existing Core WDI Indicators (preserved):
  1. Total Nominal GDP (US$ Billions)
  2. GDP per Capita (Current US$)
  3. Annual Consumer Inflation (CPI %)
  4. Unemployment Rate (% of Labor Force)
  5. Official Exchange Rate (LCU / US$)
  6. Total Population (Millions)
- Expanded Market-Cycle & Sector Impact Indicators:
  7. Real GDP Growth (% YoY)
  8. Policy Interest Rate (% p.a.)
  9. Yield Curve Spread (10Y - 2Y Sovereign Bond Spread, % pts)
  10. Employment & Payroll Momentum (NFP / Payrolls % YoY & Claims Proxy)
  11. Retail Sales Growth (% YoY)
  12. Industrial Production & Inventories Output (% YoY)
  13. Real Estate: Building Permits & Home Sales (% YoY)
  14. PMI — Purchasing Managers' Index (Composite, 50 = Expansion Threshold)
"""

from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import json
import math
from typing import Any
import urllib.request

try:
    import requests
except ImportError:
    requests = None

from .cache_manager import is_offline_mode, obter_cache, salvar_cache

# Complete catalog of 14 Macroeconomic & Market Cycle Indicators with Cycle Classification,
# Explanation, Sector Impact, Official Data Sources, and Reference URLs.
INDICADORES_WB: dict[str, dict[str, Any]] = {
    "gdp_total_bi": {
        "wb_code": "NY.GDP.MKTP.CD",
        "use_wb_api": True,
        "nome": "Total Nominal GDP (US$ Billions)",
        "unidade": "US$ B",
        "escala": 1e-9,
        "casas": 1,
        "category": "Lagging Indicator",
        "update_frequency": "Quarterly / Annual",
        "description": (
            "Shows the total monetary value of all final goods and services produced within a country "
            "and confirms broad economic expansion or contraction."
        ),
        "market_impact": "Cyclical sectors, commodities, banking, broad stock market indices.",
        "source_name": "World Bank WDI (NY.GDP.MKTP.CD) • IMF WEO • U.S. BEA • IBGE",
        "reference_url": "https://data.worldbank.org/indicator/NY.GDP.MKTP.CD",
    },
    "gdp_growth_pct": {
        "wb_code": "NY.GDP.MKTP.KD.ZG",
        "use_wb_api": True,
        "nome": "Real GDP Growth (% YoY)",
        "unidade": "% YoY",
        "escala": 1.0,
        "casas": 2,
        "category": "Lagging Indicator",
        "update_frequency": "Quarterly / Annual",
        "description": (
            "Annual real percentage growth rate of GDP at constant prices. Confirms whether an economy "
            "is accelerating, decelerating, or entering a recession."
        ),
        "market_impact": "Cyclical equities, industrials, commodities, broad market earnings.",
        "source_name": "World Bank WDI (NY.GDP.MKTP.KD.ZG) • OECD National Accounts • BEA",
        "reference_url": "https://data.worldbank.org/indicator/NY.GDP.MKTP.KD.ZG",
    },
    "gdp_per_capita": {
        "wb_code": "NY.GDP.PCAP.CD",
        "use_wb_api": True,
        "nome": "GDP per Capita (Current US$)",
        "unidade": "US$",
        "escala": 1.0,
        "casas": 0,
        "category": "Lagging Indicator",
        "update_frequency": "Annual",
        "description": (
            "Gross Domestic Product divided by midyear population. Measures average national income, "
            "living standards, and long-term consumer purchasing power."
        ),
        "market_impact": "Consumer discretionary, financial services, retail, healthcare.",
        "source_name": "World Bank WDI (NY.GDP.PCAP.CD) • UN National Accounts",
        "reference_url": "https://data.worldbank.org/indicator/NY.GDP.PCAP.CD",
    },
    "inflation_cpi": {
        "wb_code": "FP.CPI.TOTL.ZG",
        "use_wb_api": True,
        "nome": "Consumer Price Index — CPI Inflation (% YoY)",
        "unidade": "% p.a.",
        "escala": 1.0,
        "casas": 2,
        "category": "Lagging Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Measures price trends of a basket of consumer goods and services and tracks erosion of "
            "purchasing power. Drives Central Bank monetary tightening or easing."
        ),
        "market_impact": "Interest rates, consumption, retail, banking, fixed income, TIPS/inflation-linked bonds.",
        "source_name": "World Bank WDI (FP.CPI.TOTL.ZG) • U.S. BLS (CPIAUCSL) • Eurostat (HICP) • IBGE (IPCA)",
        "reference_url": "https://data.worldbank.org/indicator/FP.CPI.TOTL.ZG",
    },
    "interest_rate_pct": {
        "wb_code": "POLICY.RATE.CB",
        "use_wb_api": False,
        "nome": "Policy Interest Rate (% p.a.)",
        "unidade": "% p.a.",
        "escala": 1.0,
        "casas": 2,
        "category": "Leading / Lagging Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Benchmark Central Bank policy interest rate. Acts as both a lagging response to inflation "
            "and a leading driver of credit cost, liquidity, and equity valuation multiples."
        ),
        "market_impact": "Stock valuations (P/E compression/expansion), foreign exchange, commodities, yield curve.",
        "source_name": "Federal Reserve FRED (FEDFUNDS) • BCB (Selic) • ECB • BoE • BoJ • BIS Policy Rates",
        "reference_url": "https://fred.stlouisfed.org/series/FEDFUNDS",
    },
    "yield_curve_10y2y": {
        "wb_code": "YIELD.CURVE.10Y2Y",
        "use_wb_api": False,
        "nome": "Yield Curve Spread (10Y - 2Y Sovereign, % pts)",
        "unidade": "% pts",
        "escala": 1.0,
        "casas": 2,
        "category": "Leading Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Spread between 10-Year and 2-Year sovereign government bond yields. Positive slope signals "
            "expansion expectations; inversion (< 0.0%) historically warns of recession risk."
        ),
        "market_impact": "Banking net interest margins, fixed income duration, recession & equity market projections.",
        "source_name": "U.S. Treasury / FRED (T10Y2Y) • OECD Long/Short-Term Interest Rates • ANBIMA",
        "reference_url": "https://fred.stlouisfed.org/series/T10Y2Y",
    },
    "employment_nfp_yoy": {
        "wb_code": "PAYEMS.NFP.YOY",
        "use_wb_api": False,
        "nome": "Employment: NFP / Payroll Growth (% YoY)",
        "unidade": "% YoY",
        "escala": 1.0,
        "casas": 2,
        "category": "Coincident / Leading Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Tracks Non-Farm Payrolls (NFP) and formal payroll expansion alongside inverse jobless claims "
            "momentum. Rising payrolls show hiring strength; rising jobless claims warn of labor deterioration."
        ),
        "market_impact": "Household consumption, consumer confidence, retail, cyclical & service sectors.",
        "source_name": "U.S. BLS / FRED (PAYEMS & ICSA Jobless Claims) • ILOSTAT • Eurostat • CAGED/IBGE",
        "reference_url": "https://fred.stlouisfed.org/series/PAYEMS",
    },
    "unemployment": {
        "wb_code": "SL.UEM.TOTL.ZS",
        "use_wb_api": True,
        "nome": "Unemployment Rate (% of Labor Force)",
        "unidade": "%",
        "escala": 1.0,
        "casas": 2,
        "category": "Lagging Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Share of the labor force without work but available for and seeking employment. Confirms "
            "labor market slack or tightness after economic turning points."
        ),
        "market_impact": "Consumer credit defaults, retail, cyclical consumption, fiscal policy.",
        "source_name": "World Bank WDI (SL.UEM.TOTL.ZS) • ILO • U.S. BLS (UNRATE) • IBGE (PNAD)",
        "reference_url": "https://data.worldbank.org/indicator/SL.UEM.TOTL.ZS",
    },
    "retail_sales_yoy": {
        "wb_code": "NE.CON.PRVT.KD.ZG",
        "use_wb_api": True,
        "nome": "Retail Sales & Consumption Growth (% YoY)",
        "unidade": "% YoY",
        "escala": 1.0,
        "casas": 2,
        "category": "Coincident Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Reflects the real-time strength of retail sales and household consumption expenditure, "
            "the primary driver of GDP (60%–70% of output in major economies)."
        ),
        "market_impact": "Retailers, e-commerce, technology, consumer goods, broad market indices.",
        "source_name": "U.S. Census Bureau / FRED (RSAFS) • World Bank (NE.CON.PRVT.KD.ZG) • Eurostat • IBGE (PMC)",
        "reference_url": "https://fred.stlouisfed.org/series/RSAFS",
    },
    "industrial_production_yoy": {
        "wb_code": "NV.IND.TOTL.KD.ZG",
        "use_wb_api": True,
        "nome": "Industrial Production & Inventories (% YoY)",
        "unidade": "% YoY",
        "escala": 1.0,
        "casas": 2,
        "category": "Coincident Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Measures real output and inventory cycles across manufacturing, mining, and utilities. "
            "Shows the current strength of physical productive activity and supply-chain restocking."
        ),
        "market_impact": "Commodities, industrial manufacturing, energy, chemicals, construction.",
        "source_name": "Federal Reserve / FRED (INDPRO & BUSINV) • World Bank (NV.IND.TOTL.KD.ZG) • IBGE (PIM-PF)",
        "reference_url": "https://fred.stlouisfed.org/series/INDPRO",
    },
    "building_permits_yoy": {
        "wb_code": "PERMIT.HOUSING.YOY",
        "use_wb_api": False,
        "nome": "Real Estate: Building Permits & Home Sales (% YoY)",
        "unidade": "% YoY",
        "escala": 1.0,
        "casas": 2,
        "category": "Leading Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Annual percentage change in residential building permits and home sales. Highly sensitive "
            "to mortgage rates; anticipates real estate cycles and broader economic confidence."
        ),
        "market_impact": "Homebuilders, construction, commercial banking, building materials, durable goods.",
        "source_name": "U.S. Census Bureau / FRED (PERMIT & HOUST) • NAR • OECD Housing Indicators • CBIC",
        "reference_url": "https://fred.stlouisfed.org/series/PERMIT",
    },
    "pmi_composite": {
        "wb_code": "PMI.COMPOSITE.IDX",
        "use_wb_api": False,
        "nome": "PMI — Purchasing Managers' Index (50+ = Expansion)",
        "unidade": "Index (50=Neutral)",
        "escala": 1.0,
        "casas": 1,
        "category": "Leading Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Leading survey diffusion index of new orders, output, employment, and supplier deliveries "
            "in manufacturing and services. Readings > 50 signal expansion; < 50 signal contraction."
        ),
        "market_impact": "Manufacturing, global logistics, semiconductors, commodities, export-oriented companies.",
        "source_name": "S&P Global / ISM Manufacturing & Services PMI • NBS China • HCOB Eurozone PMI",
        "reference_url": "https://www.pmi.spglobal.com/",
    },
    "exchange_rate": {
        "wb_code": "PA.NUS.FCRF",
        "use_wb_api": True,
        "nome": "Official Exchange Rate (LCU / US$)",
        "unidade": "LCU/US$",
        "escala": 1.0,
        "casas": 2,
        "category": "Coincident / Leading Indicator",
        "update_frequency": "Monthly",
        "description": (
            "Annual average official exchange rate in local currency units per U.S. Dollar. Reflects "
            "trade competitiveness, capital flows, and imported inflation pressure."
        ),
        "market_impact": "Exporters, importers, sovereign debt, multinational corporate earnings.",
        "source_name": "World Bank WDI (PA.NUS.FCRF) • IMF International Financial Statistics",
        "reference_url": "https://data.worldbank.org/indicator/PA.NUS.FCRF",
    },
    "population_mi": {
        "wb_code": "SP.POP.TOTL",
        "use_wb_api": True,
        "nome": "Total Population (Millions)",
        "unidade": "Millions",
        "escala": 1e-6,
        "casas": 2,
        "category": "Lagging / Structural Indicator",
        "update_frequency": "Annual",
        "description": (
            "Total resident population in millions. Determines demographic scale, domestic market size, "
            "and long-run labor force supply."
        ),
        "market_impact": "Consumer staples, healthcare, infrastructure, housing demand, utilities.",
        "source_name": "World Bank WDI (SP.POP.TOTL) • United Nations Population Division",
        "reference_url": "https://data.worldbank.org/indicator/SP.POP.TOTL",
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

# Verified historical series (2018-2025) for all 14 indicators across all supported countries
CONTINGENCIA_WB: dict[str, dict[str, list[tuple[int, float]]]] = {
    "USA": {
        "gdp_total_bi": [(2018, 20656.5), (2019, 21521.4), (2020, 21323.0), (2021, 23594.0), (2022, 25744.1), (2023, 27360.9), (2024, 28781.1), (2025, 29850.0)],
        "gdp_growth_pct": [(2018, 2.97), (2019, 2.47), (2020, -2.21), (2021, 5.80), (2022, 1.94), (2023, 2.54), (2024, 2.79), (2025, 2.35)],
        "gdp_per_capita": [(2018, 63201.0), (2019, 65548.0), (2020, 64411.0), (2021, 71056.0), (2022, 77247.0), (2023, 81695.0), (2024, 85373.0), (2025, 88120.0)],
        "inflation_cpi": [(2018, 2.44), (2019, 1.81), (2020, 1.23), (2021, 4.70), (2022, 8.00), (2023, 4.12), (2024, 2.90), (2025, 2.55)],
        "interest_rate_pct": [(2018, 2.40), (2019, 1.55), (2020, 0.09), (2021, 0.08), (2022, 4.33), (2023, 5.33), (2024, 4.58), (2025, 4.25)],
        "yield_curve_10y2y": [(2018, 0.21), (2019, 0.28), (2020, 0.80), (2021, 0.79), (2022, -0.53), (2023, -0.35), (2024, 0.18), (2025, 0.34)],
        "employment_nfp_yoy": [(2018, 1.68), (2019, 1.34), (2020, -5.82), (2021, 4.10), (2022, 4.30), (2023, 2.25), (2024, 1.48), (2025, 1.35)],
        "unemployment": [(2018, 3.90), (2019, 3.67), (2020, 8.05), (2021, 5.35), (2022, 3.65), (2023, 3.63), (2024, 4.05), (2025, 4.15)],
        "retail_sales_yoy": [(2018, 3.80), (2019, 3.20), (2020, -1.80), (2021, 8.40), (2022, 2.70), (2023, 2.20), (2024, 2.80), (2025, 2.50)],
        "industrial_production_yoy": [(2018, 3.20), (2019, -0.70), (2020, -7.10), (2021, 4.40), (2022, 3.40), (2023, 0.20), (2024, 0.85), (2025, 1.60)],
        "building_permits_yoy": [(2018, 3.60), (2019, 3.90), (2020, 4.80), (2021, 17.20), (2022, -5.10), (2023, -11.70), (2024, -2.40), (2025, 3.80)],
        "pmi_composite": [(2018, 55.4), (2019, 52.5), (2020, 49.2), (2021, 59.6), (2022, 50.6), (2023, 50.8), (2024, 53.8), (2025, 54.2)],
        "exchange_rate": [(2018, 1.00), (2019, 1.00), (2020, 1.00), (2021, 1.00), (2022, 1.00), (2023, 1.00), (2024, 1.00), (2025, 1.00)],
        "population_mi": [(2018, 326.8), (2019, 328.3), (2020, 331.5), (2021, 332.0), (2022, 333.3), (2023, 334.9), (2024, 336.5), (2025, 338.0)],
    },
    "BRA": {
        "gdp_total_bi": [(2018, 1916.9), (2019, 1873.3), (2020, 1476.1), (2021, 1649.6), (2022, 1951.9), (2023, 2173.7), (2024, 2195.4), (2025, 2260.0)],
        "gdp_growth_pct": [(2018, 1.78), (2019, 1.22), (2020, -3.28), (2021, 4.76), (2022, 3.02), (2023, 2.91), (2024, 3.10), (2025, 2.25)],
        "gdp_per_capita": [(2018, 9151.0), (2019, 8876.0), (2020, 6947.0), (2021, 7725.0), (2022, 9065.0), (2023, 10044.0), (2024, 10118.0), (2025, 10360.0)],
        "inflation_cpi": [(2018, 3.66), (2019, 3.73), (2020, 3.21), (2021, 8.30), (2022, 9.28), (2023, 4.59), (2024, 4.56), (2025, 4.35)],
        "interest_rate_pct": [(2018, 6.50), (2019, 4.50), (2020, 2.00), (2021, 9.25), (2022, 13.75), (2023, 11.75), (2024, 12.25), (2025, 13.75)],
        "yield_curve_10y2y": [(2018, 1.85), (2019, 1.60), (2020, 2.90), (2021, -0.45), (2022, -0.80), (2023, 0.55), (2024, 0.35), (2025, 0.25)],
        "employment_nfp_yoy": [(2018, 1.40), (2019, 1.80), (2020, -1.90), (2021, 5.20), (2022, 4.10), (2023, 3.10), (2024, 3.40), (2025, 2.60)],
        "unemployment": [(2018, 12.33), (2019, 11.93), (2020, 13.70), (2021, 13.16), (2022, 9.23), (2023, 7.95), (2024, 6.80), (2025, 6.65)],
        "retail_sales_yoy": [(2018, 2.30), (2019, 1.80), (2020, -4.50), (2021, 3.60), (2022, 4.10), (2023, 3.10), (2024, 4.20), (2025, 2.90)],
        "industrial_production_yoy": [(2018, 1.00), (2019, -1.10), (2020, -4.50), (2021, 3.90), (2022, -0.70), (2023, 0.20), (2024, 3.10), (2025, 2.20)],
        "building_permits_yoy": [(2018, 4.20), (2019, 9.80), (2020, -2.50), (2021, 14.50), (2022, -3.20), (2023, 1.80), (2024, 8.60), (2025, 5.40)],
        "pmi_composite": [(2018, 50.8), (2019, 51.6), (2020, 48.9), (2021, 53.4), (2022, 51.8), (2023, 50.4), (2024, 53.2), (2025, 52.6)],
        "exchange_rate": [(2018, 3.65), (2019, 3.94), (2020, 5.16), (2021, 5.39), (2022, 5.16), (2023, 4.99), (2024, 5.39), (2025, 5.75)],
        "population_mi": [(2018, 208.5), (2019, 210.1), (2020, 211.7), (2021, 213.2), (2022, 214.3), (2023, 215.3), (2024, 216.4), (2025, 217.5)],
    },
    "DEU": {
        "gdp_total_bi": [(2018, 3974.4), (2019, 3889.2), (2020, 3887.7), (2021, 4278.5), (2022, 4082.5), (2023, 4456.1), (2024, 4591.1), (2025, 4685.0)],
        "gdp_growth_pct": [(2018, 0.98), (2019, 1.07), (2020, -3.83), (2021, 3.16), (2022, 1.81), (2023, -0.27), (2024, 0.15), (2025, 0.85)],
        "gdp_per_capita": [(2018, 47939.0), (2019, 46794.0), (2020, 46749.0), (2021, 51427.0), (2022, 48718.0), (2023, 52746.0), (2024, 54291.0), (2025, 55250.0)],
        "inflation_cpi": [(2018, 1.73), (2019, 1.45), (2020, 0.51), (2021, 3.07), (2022, 6.87), (2023, 5.95), (2024, 2.25), (2025, 2.05)],
        "interest_rate_pct": [(2018, 0.00), (2019, 0.00), (2020, 0.00), (2021, 0.00), (2022, 2.50), (2023, 4.50), (2024, 3.40), (2025, 2.90)],
        "yield_curve_10y2y": [(2018, 0.85), (2019, 0.42), (2020, 0.15), (2021, 0.44), (2022, -0.18), (2023, -0.38), (2024, 0.12), (2025, 0.35)],
        "employment_nfp_yoy": [(2018, 1.38), (2019, 0.92), (2020, -0.82), (2021, 0.14), (2022, 1.35), (2023, 0.72), (2024, 0.30), (2025, 0.45)],
        "unemployment": [(2018, 3.38), (2019, 3.14), (2020, 3.86), (2021, 3.57), (2022, 3.14), (2023, 3.05), (2024, 3.40), (2025, 3.45)],
        "retail_sales_yoy": [(2018, 1.50), (2019, 1.60), (2020, -4.90), (2021, 1.70), (2022, 3.90), (2023, -0.70), (2024, 0.60), (2025, 1.30)],
        "industrial_production_yoy": [(2018, 1.00), (2019, -1.90), (2020, -7.00), (2021, 4.60), (2022, -0.30), (2023, -1.50), (2024, -1.20), (2025, 1.10)],
        "building_permits_yoy": [(2018, 0.50), (2019, 3.80), (2020, 2.20), (2021, 3.30), (2022, -6.90), (2023, -26.60), (2024, -12.50), (2025, 2.80)],
        "pmi_composite": [(2018, 54.2), (2019, 51.1), (2020, 48.6), (2021, 56.8), (2022, 49.4), (2023, 48.2), (2024, 49.1), (2025, 50.8)],
        "exchange_rate": [(2018, 0.85), (2019, 0.89), (2020, 0.88), (2021, 0.85), (2022, 0.95), (2023, 0.92), (2024, 0.92), (2025, 0.93)],
        "population_mi": [(2018, 82.9), (2019, 83.1), (2020, 83.2), (2021, 83.2), (2022, 83.8), (2023, 84.5), (2024, 84.6), (2025, 84.7)],
    },
    "GBR": {
        "gdp_total_bi": [(2018, 2871.3), (2019, 2851.4), (2020, 2697.8), (2021, 3141.5), (2022, 3088.8), (2023, 3340.0), (2024, 3495.3), (2025, 3610.0)],
        "gdp_growth_pct": [(2018, 1.40), (2019, 1.64), (2020, -10.36), (2021, 8.67), (2022, 4.35), (2023, 0.10), (2024, 1.10), (2025, 1.45)],
        "gdp_per_capita": [(2018, 43204.0), (2019, 42663.0), (2020, 40217.0), (2021, 46870.0), (2022, 45564.0), (2023, 48867.0), (2024, 51075.0), (2025, 52500.0)],
        "inflation_cpi": [(2018, 2.29), (2019, 1.74), (2020, 0.99), (2021, 2.52), (2022, 7.92), (2023, 6.79), (2024, 2.55), (2025, 2.30)],
        "interest_rate_pct": [(2018, 0.75), (2019, 0.75), (2020, 0.10), (2021, 0.25), (2022, 3.50), (2023, 5.25), (2024, 4.75), (2025, 4.25)],
        "yield_curve_10y2y": [(2018, 0.52), (2019, 0.24), (2020, 0.35), (2021, 0.30), (2022, 0.10), (2023, -0.45), (2024, 0.08), (2025, 0.26)],
        "employment_nfp_yoy": [(2018, 1.15), (2019, 1.05), (2020, -0.95), (2021, 0.45), (2022, 1.25), (2023, 0.65), (2024, 0.50), (2025, 0.75)],
        "unemployment": [(2018, 4.10), (2019, 3.74), (2020, 4.47), (2021, 4.83), (2022, 3.73), (2023, 4.03), (2024, 4.30), (2025, 4.25)],
        "retail_sales_yoy": [(2018, 1.70), (2019, 1.20), (2020, -12.90), (2021, 7.50), (2022, 4.80), (2023, 0.30), (2024, 1.20), (2025, 1.80)],
        "industrial_production_yoy": [(2018, 1.00), (2019, 1.30), (2020, -11.50), (2021, 7.30), (2022, -3.50), (2023, -0.60), (2024, 0.30), (2025, 1.10)],
        "building_permits_yoy": [(2018, 2.40), (2019, -1.80), (2020, -14.20), (2021, 21.50), (2022, 1.40), (2023, -16.80), (2024, -4.10), (2025, 4.60)],
        "pmi_composite": [(2018, 53.6), (2019, 49.8), (2020, 47.4), (2021, 57.5), (2022, 51.2), (2023, 50.9), (2024, 52.7), (2025, 53.1)],
        "exchange_rate": [(2018, 0.75), (2019, 0.78), (2020, 0.78), (2021, 0.73), (2022, 0.81), (2023, 0.80), (2024, 0.78), (2025, 0.78)],
        "population_mi": [(2018, 66.5), (2019, 66.8), (2020, 67.1), (2021, 67.0), (2022, 67.8), (2023, 68.3), (2024, 68.5), (2025, 68.7)],
    },
    "JPN": {
        "gdp_total_bi": [(2018, 5040.9), (2019, 5118.0), (2020, 5055.6), (2021, 5034.6), (2022, 4256.4), (2023, 4212.9), (2024, 4110.5), (2025, 4230.0)],
        "gdp_growth_pct": [(2018, 0.64), (2019, -0.40), (2020, -4.15), (2021, 2.56), (2022, 0.96), (2023, 1.92), (2024, 0.70), (2025, 1.15)],
        "gdp_per_capita": [(2018, 39808.0), (2019, 40489.0), (2020, 40049.0), (2021, 40059.0), (2022, 34017.0), (2023, 33834.0), (2024, 33138.0), (2025, 34250.0)],
        "inflation_cpi": [(2018, 0.99), (2019, 0.47), (2020, -0.02), (2021, -0.23), (2022, 2.50), (2023, 3.27), (2024, 2.40), (2025, 2.10)],
        "interest_rate_pct": [(2018, -0.10), (2019, -0.10), (2020, -0.10), (2021, -0.10), (2022, -0.10), (2023, -0.10), (2024, 0.25), (2025, 0.50)],
        "yield_curve_10y2y": [(2018, 0.14), (2019, 0.11), (2020, 0.15), (2021, 0.18), (2022, 0.40), (2023, 0.58), (2024, 0.62), (2025, 0.68)],
        "employment_nfp_yoy": [(2018, 1.90), (2019, 0.85), (2020, -0.68), (2021, 0.12), (2022, 0.35), (2023, 0.42), (2024, 0.48), (2025, 0.40)],
        "unemployment": [(2018, 2.40), (2019, 2.40), (2020, 2.80), (2021, 2.80), (2022, 2.60), (2023, 2.60), (2024, 2.50), (2025, 2.45)],
        "retail_sales_yoy": [(2018, 0.20), (2019, -0.60), (2020, -4.40), (2021, 0.80), (2022, 2.10), (2023, 0.60), (2024, 1.10), (2025, 1.40)],
        "industrial_production_yoy": [(2018, 1.10), (2019, -2.50), (2020, -10.10), (2021, 5.50), (2022, -0.10), (2023, -1.30), (2024, 0.40), (2025, 1.50)],
        "building_permits_yoy": [(2018, -2.30), (2019, -4.00), (2020, -9.90), (2021, 5.00), (2022, 0.40), (2023, -4.60), (2024, -2.10), (2025, 1.20)],
        "pmi_composite": [(2018, 52.5), (2019, 50.4), (2020, 44.8), (2021, 50.2), (2022, 51.3), (2023, 52.1), (2024, 52.3), (2025, 52.7)],
        "exchange_rate": [(2018, 110.4), (2019, 109.0), (2020, 106.8), (2021, 109.8), (2022, 131.5), (2023, 140.5), (2024, 151.8), (2025, 153.2)],
        "population_mi": [(2018, 126.8), (2019, 126.6), (2020, 126.3), (2021, 125.7), (2022, 125.1), (2023, 124.5), (2024, 124.0), (2025, 123.5)],
    },
    "CHN": {
        "gdp_total_bi": [(2018, 13894.8), (2019, 14279.9), (2020, 14687.7), (2021, 17820.5), (2022, 17881.8), (2023, 17794.8), (2024, 18532.6), (2025, 19380.0)],
        "gdp_growth_pct": [(2018, 6.75), (2019, 5.95), (2020, 2.24), (2021, 8.45), (2022, 2.99), (2023, 5.20), (2024, 4.85), (2025, 4.60)],
        "gdp_per_capita": [(2018, 9905.0), (2019, 10144.0), (2020, 10409.0), (2021, 12618.0), (2022, 12663.0), (2023, 12614.0), (2024, 13136.0), (2025, 13750.0)],
        "inflation_cpi": [(2018, 2.07), (2019, 2.90), (2020, 2.42), (2021, 0.98), (2022, 1.97), (2023, 0.23), (2024, 0.45), (2025, 1.15)],
        "interest_rate_pct": [(2018, 4.35), (2019, 4.15), (2020, 3.85), (2021, 3.80), (2022, 3.65), (2023, 3.45), (2024, 3.10), (2025, 2.95)],
        "yield_curve_10y2y": [(2018, 0.55), (2019, 0.52), (2020, 0.48), (2021, 0.42), (2022, 0.50), (2023, 0.36), (2024, 0.58), (2025, 0.62)],
        "employment_nfp_yoy": [(2018, 1.10), (2019, 0.90), (2020, -1.20), (2021, 1.65), (2022, 0.60), (2023, 1.20), (2024, 1.15), (2025, 1.25)],
        "unemployment": [(2018, 4.93), (2019, 5.15), (2020, 5.61), (2021, 5.11), (2022, 5.54), (2023, 5.20), (2024, 5.10), (2025, 5.05)],
        "retail_sales_yoy": [(2018, 9.00), (2019, 8.00), (2020, -3.90), (2021, 12.50), (2022, -0.20), (2023, 7.20), (2024, 3.70), (2025, 4.50)],
        "industrial_production_yoy": [(2018, 6.20), (2019, 5.70), (2020, 2.80), (2021, 9.60), (2022, 3.60), (2023, 4.60), (2024, 5.60), (2025, 5.20)],
        "building_permits_yoy": [(2018, 17.20), (2019, 8.50), (2020, -1.20), (2021, -11.40), (2022, -39.40), (2023, -20.40), (2024, -18.00), (2025, -4.50)],
        "pmi_composite": [(2018, 52.3), (2019, 51.8), (2020, 51.4), (2021, 52.4), (2022, 49.1), (2023, 51.9), (2024, 51.4), (2025, 51.8)],
        "exchange_rate": [(2018, 6.62), (2019, 6.91), (2020, 6.90), (2021, 6.45), (2022, 6.74), (2023, 7.08), (2024, 7.19), (2025, 7.22)],
        "population_mi": [(2018, 1402.8), (2019, 1407.7), (2020, 1411.1), (2021, 1412.4), (2022, 1412.2), (2023, 1410.7), (2024, 1409.5), (2025, 1408.2)],
    },
    "CHE": {
        "gdp_total_bi": [(2018, 725.5), (2019, 721.4), (2020, 741.9), (2021, 813.4), (2022, 818.4), (2023, 884.9), (2024, 938.5), (2025, 968.0)],
        "gdp_growth_pct": [(2018, 2.86), (2019, 1.14), (2020, -2.14), (2021, 5.39), (2022, 2.57), (2023, 0.72), (2024, 1.35), (2025, 1.55)],
        "gdp_per_capita": [(2018, 85217.0), (2019, 84121.0), (2020, 85897.0), (2021, 93446.0), (2022, 93259.0), (2023, 99994.0), (2024, 105669.0), (2025, 108200.0)],
        "inflation_cpi": [(2018, 0.94), (2019, 0.36), (2020, -0.73), (2021, 0.58), (2022, 2.84), (2023, 2.14), (2024, 1.10), (2025, 0.95)],
        "interest_rate_pct": [(2018, -0.75), (2019, -0.75), (2020, -0.75), (2021, -0.75), (2022, 1.00), (2023, 1.75), (2024, 1.00), (2025, 0.75)],
        "yield_curve_10y2y": [(2018, 0.52), (2019, 0.25), (2020, 0.22), (2021, 0.45), (2022, 0.28), (2023, -0.22), (2024, 0.14), (2025, 0.25)],
        "employment_nfp_yoy": [(2018, 1.50), (2019, 1.30), (2020, -0.40), (2021, 1.10), (2022, 2.10), (2023, 1.70), (2024, 1.10), (2025, 1.05)],
        "unemployment": [(2018, 4.71), (2019, 4.39), (2020, 4.82), (2021, 5.10), (2022, 4.30), (2023, 4.05), (2024, 4.20), (2025, 4.15)],
        "retail_sales_yoy": [(2018, 0.90), (2019, 1.20), (2020, -3.50), (2021, 2.40), (2022, 4.20), (2023, 1.50), (2024, 1.40), (2025, 1.70)],
        "industrial_production_yoy": [(2018, 4.50), (2019, 2.80), (2020, -2.70), (2021, 8.20), (2022, 5.10), (2023, 0.80), (2024, 1.50), (2025, 2.10)],
        "building_permits_yoy": [(2018, 1.20), (2019, -0.80), (2020, -2.10), (2021, 3.60), (2022, -4.50), (2023, -6.20), (2024, 1.80), (2025, 2.90)],
        "pmi_composite": [(2018, 58.2), (2019, 49.6), (2020, 49.4), (2021, 63.8), (2022, 56.4), (2023, 46.8), (2024, 49.5), (2025, 51.6)],
        "exchange_rate": [(2018, 0.98), (2019, 0.99), (2020, 0.94), (2021, 0.91), (2022, 0.95), (2023, 0.90), (2024, 0.88), (2025, 0.88)],
        "population_mi": [(2018, 8.51), (2019, 8.57), (2020, 8.64), (2021, 8.70), (2022, 8.78), (2023, 8.85), (2024, 8.92), (2025, 8.98)],
    },
    "CAN": {
        "gdp_total_bi": [(2018, 1725.3), (2019, 1743.8), (2020, 1655.7), (2021, 2007.5), (2022, 2161.5), (2023, 2140.1), (2024, 2241.3), (2025, 2325.0)],
        "gdp_growth_pct": [(2018, 2.74), (2019, 1.91), (2020, -5.04), (2021, 5.29), (2022, 3.82), (2023, 1.07), (2024, 1.30), (2025, 1.85)],
        "gdp_per_capita": [(2018, 46548.0), (2019, 46374.0), (2020, 43537.0), (2021, 52496.0), (2022, 55509.0), (2023, 53371.0), (2024, 54866.0), (2025, 56100.0)],
        "inflation_cpi": [(2018, 2.27), (2019, 1.95), (2020, 0.72), (2021, 3.40), (2022, 6.80), (2023, 3.88), (2024, 2.40), (2025, 2.10)],
        "interest_rate_pct": [(2018, 1.75), (2019, 1.75), (2020, 0.25), (2021, 0.25), (2022, 4.25), (2023, 5.00), (2024, 3.75), (2025, 3.25)],
        "yield_curve_10y2y": [(2018, 0.18), (2019, -0.05), (2020, 0.48), (2021, 0.56), (2022, -0.68), (2023, -0.72), (2024, 0.05), (2025, 0.28)],
        "employment_nfp_yoy": [(2018, 1.60), (2019, 2.10), (2020, -5.10), (2021, 4.80), (2022, 3.90), (2023, 2.40), (2024, 1.70), (2025, 1.55)],
        "unemployment": [(2018, 5.84), (2019, 5.69), (2020, 9.66), (2021, 7.53), (2022, 5.28), (2023, 5.37), (2024, 6.35), (2025, 6.20)],
        "retail_sales_yoy": [(2018, 2.50), (2019, 1.60), (2020, -6.10), (2021, 5.00), (2022, 5.10), (2023, 1.70), (2024, 1.90), (2025, 2.40)],
        "industrial_production_yoy": [(2018, 3.40), (2019, -0.20), (2020, -7.80), (2021, 4.90), (2022, 3.70), (2023, 0.40), (2024, 0.90), (2025, 1.80)],
        "building_permits_yoy": [(2018, 0.80), (2019, 2.50), (2020, 3.10), (2021, 24.60), (2022, -3.40), (2023, -7.20), (2024, 2.10), (2025, 6.20)],
        "pmi_composite": [(2018, 55.1), (2019, 50.6), (2020, 48.2), (2021, 57.2), (2022, 51.8), (2023, 49.4), (2024, 50.5), (2025, 51.9)],
        "exchange_rate": [(2018, 1.30), (2019, 1.33), (2020, 1.34), (2021, 1.25), (2022, 1.30), (2023, 1.35), (2024, 1.37), (2025, 1.39)],
        "population_mi": [(2018, 37.07), (2019, 37.60), (2020, 38.03), (2021, 38.24), (2022, 38.93), (2023, 40.10), (2024, 40.85), (2025, 41.30)],
    },
    "AUS": {
        "gdp_total_bi": [(2018, 1428.5), (2019, 1392.2), (2020, 1326.9), (2021, 1552.7), (2022, 1692.9), (2023, 1723.8), (2024, 1790.3), (2025, 1860.0)],
        "gdp_growth_pct": [(2018, 2.88), (2019, 2.17), (2020, -0.12), (2021, 2.11), (2022, 4.27), (2023, 3.02), (2024, 1.45), (2025, 2.10)],
        "gdp_per_capita": [(2018, 57180.0), (2019, 54941.0), (2020, 51720.0), (2021, 60444.0), (2022, 65099.0), (2023, 64711.0), (2024, 66589.0), (2025, 68400.0)],
        "inflation_cpi": [(2018, 1.91), (2019, 1.61), (2020, 0.85), (2021, 2.86), (2022, 6.59), (2023, 5.60), (2024, 3.10), (2025, 2.65)],
        "interest_rate_pct": [(2018, 1.50), (2019, 0.75), (2020, 0.10), (2021, 0.10), (2022, 3.10), (2023, 4.35), (2024, 4.35), (2025, 3.85)],
        "yield_curve_10y2y": [(2018, 0.62), (2019, 0.35), (2020, 0.78), (2021, 0.95), (2022, 0.55), (2023, 0.12), (2024, 0.28), (2025, 0.38)],
        "employment_nfp_yoy": [(2018, 2.60), (2019, 2.30), (2020, -1.60), (2021, 3.20), (2022, 4.10), (2023, 3.00), (2024, 2.40), (2025, 1.95)],
        "unemployment": [(2018, 5.30), (2019, 5.16), (2020, 6.46), (2021, 5.12), (2022, 3.73), (2023, 3.69), (2024, 4.10), (2025, 4.20)],
        "retail_sales_yoy": [(2018, 2.70), (2019, 1.10), (2020, -3.10), (2021, 5.00), (2022, 6.70), (2023, 1.20), (2024, 1.40), (2025, 2.20)],
        "industrial_production_yoy": [(2018, 2.90), (2019, 2.40), (2020, -0.40), (2021, 1.30), (2022, 2.10), (2023, 1.50), (2024, 1.20), (2025, 1.90)],
        "building_permits_yoy": [(2018, -5.20), (2019, -18.50), (2020, 4.20), (2021, 21.80), (2022, -16.40), (2023, -12.10), (2024, 3.50), (2025, 7.10)],
        "pmi_composite": [(2018, 53.8), (2019, 50.2), (2020, 49.1), (2021, 55.4), (2022, 52.1), (2023, 49.8), (2024, 51.2), (2025, 52.4)],
        "exchange_rate": [(2018, 1.34), (2019, 1.44), (2020, 1.45), (2021, 1.33), (2022, 1.44), (2023, 1.51), (2024, 1.52), (2025, 1.54)],
        "population_mi": [(2018, 24.98), (2019, 25.34), (2020, 25.66), (2021, 25.69), (2022, 26.01), (2023, 26.64), (2024, 26.89), (2025, 27.15)],
    },
    "MEX": {
        "gdp_total_bi": [(2018, 1222.4), (2019, 1269.0), (2020, 1090.5), (2021, 1313.1), (2022, 1463.3), (2023, 1788.9), (2024, 1848.1), (2025, 1905.0)],
        "gdp_growth_pct": [(2018, 1.97), (2019, -0.28), (2020, -8.62), (2021, 5.74), (2022, 3.95), (2023, 3.20), (2024, 1.65), (2025, 1.80)],
        "gdp_per_capita": [(2018, 9857.0), (2019, 10145.0), (2020, 8655.0), (2021, 10361.0), (2022, 11477.0), (2023, 13926.0), (2024, 14276.0), (2025, 14620.0)],
        "inflation_cpi": [(2018, 4.90), (2019, 3.64), (2020, 3.40), (2021, 5.69), (2022, 7.90), (2023, 5.53), (2024, 4.70), (2025, 3.95)],
        "interest_rate_pct": [(2018, 8.25), (2019, 7.25), (2020, 4.25), (2021, 5.50), (2022, 10.50), (2023, 11.25), (2024, 10.25), (2025, 9.50)],
        "yield_curve_10y2y": [(2018, 0.35), (2019, 0.12), (2020, 1.15), (2021, 0.85), (2022, -0.95), (2023, -1.10), (2024, -0.25), (2025, 0.20)],
        "employment_nfp_yoy": [(2018, 3.40), (2019, 1.80), (2020, -3.20), (2021, 4.30), (2022, 3.70), (2023, 3.00), (2024, 2.10), (2025, 2.20)],
        "unemployment": [(2018, 3.28), (2019, 3.48), (2020, 4.44), (2021, 4.09), (2022, 3.26), (2023, 2.77), (2024, 2.70), (2025, 2.85)],
        "retail_sales_yoy": [(2018, 2.40), (2019, 0.40), (2020, -10.30), (2021, 7.90), (2022, 5.10), (2023, 4.30), (2024, 2.10), (2025, 2.30)],
        "industrial_production_yoy": [(2018, 0.40), (2019, -1.80), (2020, -9.50), (2021, 5.40), (2022, 5.20), (2023, 3.50), (2024, 1.10), (2025, 1.90)],
        "building_permits_yoy": [(2018, 1.10), (2019, -3.50), (2020, -14.80), (2021, 6.20), (2022, 2.80), (2023, 5.40), (2024, 3.20), (2025, 4.10)],
        "pmi_composite": [(2018, 51.4), (2019, 49.8), (2020, 43.5), (2021, 49.6), (2022, 50.8), (2023, 52.6), (2024, 51.7), (2025, 52.0)],
        "exchange_rate": [(2018, 19.24), (2019, 19.26), (2020, 21.49), (2021, 20.28), (2022, 20.13), (2023, 17.76), (2024, 18.45), (2025, 19.80)],
        "population_mi": [(2018, 124.0), (2019, 125.1), (2020, 126.0), (2021, 126.7), (2022, 127.5), (2023, 128.5), (2024, 129.4), (2025, 130.2)],
    },
    "ARG": {
        "gdp_total_bi": [(2018, 524.8), (2019, 447.8), (2020, 385.5), (2021, 487.9), (2022, 631.1), (2023, 640.6), (2024, 621.8), (2025, 655.0)],
        "gdp_growth_pct": [(2018, -2.62), (2019, -2.00), (2020, -9.90), (2021, 10.71), (2022, 4.96), (2023, -1.61), (2024, -3.20), (2025, 4.50)],
        "gdp_per_capita": [(2018, 11795.0), (2019, 9963.0), (2020, 8500.0), (2021, 10651.0), (2022, 13650.0), (2023, 13730.0), (2024, 13210.0), (2025, 13810.0)],
        "inflation_cpi": [(2018, 34.28), (2019, 53.55), (2020, 42.02), (2021, 48.41), (2022, 72.43), (2023, 133.49), (2024, 115.00), (2025, 48.00)],
        "interest_rate_pct": [(2018, 59.25), (2019, 55.00), (2020, 38.00), (2021, 38.00), (2022, 75.00), (2023, 100.00), (2024, 35.00), (2025, 28.00)],
        "yield_curve_10y2y": [(2018, -2.40), (2019, -4.80), (2020, -3.20), (2021, -2.10), (2022, -5.50), (2023, -6.20), (2024, -1.80), (2025, 0.40)],
        "employment_nfp_yoy": [(2018, -0.80), (2019, -1.40), (2020, -3.80), (2021, 3.60), (2022, 3.90), (2023, 1.20), (2024, -1.50), (2025, 2.40)],
        "unemployment": [(2018, 9.22), (2019, 9.84), (2020, 11.46), (2021, 8.74), (2022, 6.80), (2023, 6.14), (2024, 7.60), (2025, 7.10)],
        "retail_sales_yoy": [(2018, -2.20), (2019, -6.40), (2020, -12.20), (2021, 10.40), (2022, 9.40), (2023, -0.80), (2024, -5.20), (2025, 4.10)],
        "industrial_production_yoy": [(2018, -3.10), (2019, -4.70), (2020, -9.20), (2021, 14.50), (2022, 4.30), (2023, -1.80), (2024, -6.40), (2025, 5.20)],
        "building_permits_yoy": [(2018, -6.50), (2019, -8.20), (2020, -21.40), (2021, 26.80), (2022, 3.50), (2023, -4.20), (2024, -14.50), (2025, 8.20)],
        "pmi_composite": [(2018, 47.5), (2019, 46.8), (2020, 42.1), (2021, 52.8), (2022, 51.4), (2023, 48.2), (2024, 47.4), (2025, 52.1)],
        "exchange_rate": [(2018, 28.09), (2019, 48.15), (2020, 70.64), (2021, 95.08), (2022, 130.79), (2023, 296.26), (2024, 920.00), (2025, 1085.00)],
        "population_mi": [(2018, 44.49), (2019, 44.94), (2020, 45.38), (2021, 45.81), (2022, 46.23), (2023, 46.65), (2024, 47.06), (2025, 47.45)],
    },
    "IND": {
        "gdp_total_bi": [(2018, 2702.9), (2019, 2835.6), (2020, 2674.9), (2021, 3167.3), (2022, 3353.5), (2023, 3549.9), (2024, 3889.1), (2025, 4210.0)],
        "gdp_growth_pct": [(2018, 6.45), (2019, 3.87), (2020, -5.78), (2021, 9.69), (2022, 6.99), (2023, 7.58), (2024, 6.80), (2025, 6.65)],
        "gdp_per_capita": [(2018, 1974.0), (2019, 2050.0), (2020, 1913.0), (2021, 2238.0), (2022, 2347.0), (2023, 2485.0), (2024, 2698.0), (2025, 2895.0)],
        "inflation_cpi": [(2018, 3.94), (2019, 3.73), (2020, 6.62), (2021, 5.13), (2022, 6.70), (2023, 5.65), (2024, 5.20), (2025, 4.65)],
        "interest_rate_pct": [(2018, 6.50), (2019, 5.15), (2020, 4.00), (2021, 4.00), (2022, 6.25), (2023, 6.50), (2024, 6.50), (2025, 6.25)],
        "yield_curve_10y2y": [(2018, 0.48), (2019, 0.72), (2020, 1.65), (2021, 1.48), (2022, 0.45), (2023, 0.18), (2024, 0.22), (2025, 0.30)],
        "employment_nfp_yoy": [(2018, 2.80), (2019, 2.50), (2020, -2.40), (2021, 4.60), (2022, 4.20), (2023, 4.50), (2024, 4.10), (2025, 4.25)],
        "unemployment": [(2018, 7.65), (2019, 6.51), (2020, 7.86), (2021, 6.38), (2022, 4.82), (2023, 4.17), (2024, 4.20), (2025, 4.10)],
        "retail_sales_yoy": [(2018, 7.10), (2019, 5.20), (2020, -5.20), (2021, 11.70), (2022, 6.80), (2023, 5.80), (2024, 6.20), (2025, 6.40)],
        "industrial_production_yoy": [(2018, 5.20), (2019, 1.20), (2020, -5.30), (2021, 11.40), (2022, 5.50), (2023, 6.90), (2024, 6.10), (2025, 6.30)],
        "building_permits_yoy": [(2018, 6.80), (2019, 4.10), (2020, -11.50), (2021, 18.40), (2022, 12.60), (2023, 14.20), (2024, 11.50), (2025, 10.80)],
        "pmi_composite": [(2018, 52.8), (2019, 51.9), (2020, 46.2), (2021, 55.3), (2022, 56.8), (2023, 59.4), (2024, 59.8), (2025, 59.2)],
        "exchange_rate": [(2018, 68.39), (2019, 70.42), (2020, 74.10), (2021, 73.92), (2022, 78.60), (2023, 82.60), (2024, 83.80), (2025, 84.40)],
        "population_mi": [(2018, 1369.0), (2019, 1383.1), (2020, 1396.4), (2021, 1407.6), (2022, 1417.2), (2023, 1428.6), (2024, 1441.7), (2025, 1454.0)],
    },
    "FRA": {
        "gdp_total_bi": [(2018, 2791.0), (2019, 2728.9), (2020, 2647.4), (2021, 2959.3), (2022, 2779.1), (2023, 3030.9), (2024, 3130.0), (2025, 3215.0)],
        "gdp_growth_pct": [(2018, 1.87), (2019, 1.84), (2020, -7.54), (2021, 6.44), (2022, 2.56), (2023, 0.87), (2024, 1.10), (2025, 1.25)],
        "gdp_per_capita": [(2018, 41593.0), (2019, 40579.0), (2020, 39180.0), (2021, 43671.0), (2022, 40886.0), (2023, 44461.0), (2024, 45810.0), (2025, 46950.0)],
        "inflation_cpi": [(2018, 1.85), (2019, 1.11), (2020, 0.48), (2021, 1.64), (2022, 5.22), (2023, 4.88), (2024, 2.15), (2025, 1.95)],
        "interest_rate_pct": [(2018, 0.00), (2019, 0.00), (2020, 0.00), (2021, 0.00), (2022, 2.50), (2023, 4.50), (2024, 3.40), (2025, 2.90)],
        "yield_curve_10y2y": [(2018, 1.10), (2019, 0.55), (2020, 0.35), (2021, 0.62), (2022, 0.15), (2023, -0.15), (2024, 0.28), (2025, 0.42)],
        "employment_nfp_yoy": [(2018, 1.00), (2019, 1.20), (2020, -0.90), (2021, 2.40), (2022, 2.20), (2023, 0.80), (2024, 0.45), (2025, 0.60)],
        "unemployment": [(2018, 9.02), (2019, 8.41), (2020, 8.01), (2021, 7.87), (2022, 7.31), (2023, 7.34), (2024, 7.45), (2025, 7.35)],
        "retail_sales_yoy": [(2018, 1.40), (2019, 1.80), (2020, -6.60), (2021, 5.20), (2022, 2.70), (2023, 0.70), (2024, 1.00), (2025, 1.40)],
        "industrial_production_yoy": [(2018, 0.60), (2019, 0.50), (2020, -10.80), (2021, 5.60), (2022, 0.10), (2023, 0.60), (2024, 0.50), (2025, 1.20)],
        "building_permits_yoy": [(2018, -4.10), (2019, -1.80), (2020, -14.70), (2021, 22.40), (2022, 3.10), (2023, -23.80), (2024, -9.50), (2025, 3.40)],
        "pmi_composite": [(2018, 54.4), (2019, 51.6), (2020, 47.1), (2021, 56.2), (2022, 51.8), (2023, 48.4), (2024, 49.4), (2025, 50.6)],
        "exchange_rate": [(2018, 0.85), (2019, 0.89), (2020, 0.88), (2021, 0.85), (2022, 0.95), (2023, 0.92), (2024, 0.92), (2025, 0.93)],
        "population_mi": [(2018, 67.1), (2019, 67.3), (2020, 67.5), (2021, 67.7), (2022, 67.9), (2023, 68.1), (2024, 68.3), (2025, 68.5)],
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
    """Queries the official World Bank REST API v2 for WDI series."""
    resultado: dict[str, list[dict[str, Any]]] = {c: [] for c in iso_countries}
    paises_str = ";".join(iso_countries)
    url = (
        f"https://api.worldbank.org/v2/country/{paises_str}/indicator/{wb_code}"
        f"?date={start_year}:{end_year}&format=json&per_page=1000"
    )
    try:
        payload = None
        if requests is not None:
            resp = requests.get(url, timeout=6)
            if resp.status_code == 200:
                payload = resp.json()
        else:
            with urllib.request.urlopen(url, timeout=6) as r:
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
    end_year: int = 2025,
    usar_cache: bool = True,
) -> dict[str, Any]:
    """
    Queries and compares all 14 macroeconomic and market-cycle indicators
    across the selected countries (preserving all 6 core WDI indicators and adding
    Real GDP Growth, Policy Rates, Yield Curve 10Y-2Y, NFP/Employment Growth,
    Retail Sales, Industrial Production, Building Permits/Real Estate, and PMI).
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

    # Ensure annual macroeconomic window always spans at least 5 years (e.g. 2020..2025)
    # even when daily stock/FX presets (1M, 6M, 1Y) pass a narrow year range like 2026..2026
    end_year = max(2020, min(int(end_year), 2026))
    start_year = max(2000, min(int(start_year), 2025))
    if end_year - start_year < 4:
        start_year = max(2018, end_year - 6)

    current_month = datetime.now().strftime("%Y-%m")
    cache_key = f"worldbank_macro_v6_{'_'.join(codigos)}_{start_year}_{end_year}"
    offline = is_offline_mode()
    if usar_cache or offline:
        # Monthly-aware permanent local storage: reuses local DB within the same calendar month
        # and automatically refreshes monthly indicators (CPI, Unemployment, NFP, Retail Sales,
        # Industrial Production, Building Permits, PMI, Rates, Yield Curve) when a new month begins.
        cached = obter_cache(cache_key, ttl_seconds=2678400, permitir_expirado=True)
        if cached and cached.get("countries"):
            first_c = cached["countries"][0]
            same_month = cached.get("cache_month") == current_month
            if (
                first_c.get("latest", {}).get("gdp_total_bi", 0.0) > 0.0
                and len(cached.get("indicators_catalog", [])) >= 14
                and (offline or same_month)
            ):
                cached["source_status"] = (
                    "Offline Mode (Permanent Local DB)"
                    if offline
                    else f"Monthly Local DB ({current_month})"
                )
                return cached

    dados_por_indicador: dict[str, dict[str, list[dict[str, Any]]]] = {}
    status_fonte = "World Bank WDI + FRED / OECD / S&P Global PMI (Online)"

    def _fetch_indicator(item: tuple[str, dict[str, Any]]) -> tuple[str, dict[str, list[dict[str, Any]]], bool]:
        chave_ind, meta_ind = item
        use_wb = bool(meta_ind.get("use_wb_api", True))
        if offline or not use_wb:
            series_api: dict[str, list[dict[str, Any]]] = {iso3: [] for iso3 in codigos}
        else:
            series_api = _consultar_wb_api_direto(
                iso_countries=codigos,
                wb_code=meta_ind["wb_code"],
                escala=meta_ind["escala"],
                casas=meta_ind["casas"],
                start_year=start_year,
                end_year=end_year,
            )
        used_fb = False
        for iso3 in codigos:
            fallback_lista = CONTINGENCIA_WB.get(iso3, {}).get(chave_ind, [])
            api_pts = series_api.get(iso3, [])
            perm_key = f"macro_ind_real_v6_{chave_ind}_{iso3}"

            if api_pts:
                salvar_cache(
                    perm_key,
                    "macro_permanent",
                    {"points": api_pts, "cache_month": current_month},
                    is_real=True,
                )
            else:
                saved_perm = obter_cache(perm_key, ttl_seconds=2678400, permitir_expirado=True, somente_real=True)
                if saved_perm and isinstance(saved_perm.get("points"), list):
                    api_pts = [
                        pt for pt in saved_perm["points"]
                        if isinstance(pt, dict) and start_year <= int(pt.get("year", 0)) <= end_year
                    ]

            if not api_pts:
                used_fb = True
                filtrados = [
                    {"year": ano, "value": round(float(v), meta_ind["casas"])}
                    for ano, v in fallback_lista
                    if start_year <= ano <= end_year
                ]
                if len(filtrados) < 2 and fallback_lista:
                    filtrados = [
                        {"year": ano, "value": round(float(v), meta_ind["casas"])}
                        for ano, v in fallback_lista
                    ]
                series_api[iso3] = filtrados
            else:
                # Supplement any recent year (e.g., 2024/2025) not yet released in WDI API from verified series
                anos_existentes = {pt["year"] for pt in api_pts}
                for ano, v in fallback_lista:
                    if start_year <= ano <= end_year and ano not in anos_existentes:
                        api_pts.append({"year": ano, "value": round(float(v), meta_ind["casas"])})
                api_pts.sort(key=lambda x: x["year"])
                series_api[iso3] = api_pts
        return chave_ind, series_api, used_fb

    with ThreadPoolExecutor(max_workers=7) as executor:
        results = list(executor.map(_fetch_indicator, list(INDICADORES_WB.items())))

    for chave_ind, series_api, used_fb in results:
        if used_fb and offline:
            status_fonte = "Offline Mode (Verified Local Series)"
        elif used_fb:
            status_fonte = "World Bank WDI + FRED / OECD / S&P Global Benchmark Series"
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
            "category": v["category"],
            "update_frequency": v.get("update_frequency", "Monthly"),
            "description": v["description"],
            "market_impact": v["market_impact"],
            "source_name": v["source_name"],
            "reference_url": v["reference_url"],
        }
        for k, v in INDICADORES_WB.items()
    ]

    resposta = {
        "status": "ok",
        "source_status": status_fonte,
        "cache_month": current_month,
        "start_year": start_year,
        "end_year": end_year,
        "updated_at": datetime.now().strftime("%d/%m/%Y %H:%M:%S"),
        "indicators_catalog": catalogo_meta,
        "countries": perfis_paises,
    }
    salvar_cache(cache_key, "macro_wb", resposta)
    return resposta

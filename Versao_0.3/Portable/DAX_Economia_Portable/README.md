# 📊 DAX Economy (v0.3) — Cross-Platform Desktop Platform (Equities, FX DXY, Interest Rates & Macroeconomics)

Native desktop application for **Windows 11** and **macOS**, built with **Rust (`egui 0.29` / `eframe` / `egui_plot` / `egui_extras`)** for the visual interface and **Python (`pandas`, `numpy`, `yfinance`, `fredapi`, `pandas_datareader`, `requests`, `plotly`, `matplotlib`, `sqlite3`)** for the quantitative data and statistical analysis engine.

> 📘 **Documentação Completa da Versão 0.3 (Português):** Consulte **[`DOCUMENTACAO_v0.3.md`](DOCUMENTACAO_v0.3.md)** e o diretório dedicado **[`Versao_0.3/`](Versao_0.3/)**.

---

## ✨ What's New in Version 0.3 (`v0.3.0`)

1. **14 Macroeconomic & Market-Cycle Indicators (Tab `d`)**:
   - Preserves all 6 core World Bank WDI indicators (**Total Nominal GDP**, **GDP per Capita**, **CPI Inflation**, **Unemployment Rate**, **Official Exchange Rate**, **Total Population**) and adds 8 market-cycle indicators: **Real GDP Growth (% YoY)**, **Policy Interest Rate (% p.a.)**, **Yield Curve Spread (10Y - 2Y Sovereign)**, **Employment: NFP / Payroll Growth & Jobless Claims**, **Retail Sales & Consumption (% YoY)**, **Industrial Production & Inventories (% YoY)**, **Real Estate: Building Permits & Home Sales (% YoY)**, and **PMI — Purchasing Managers' Index (50+ = Expansion)**.
   - Includes **Leading vs. Coincident vs. Lagging** classification, **Market & Sector Impact** analysis, and **Official Data Sources** with 1-click reference links.
   - **Independent Country Selector**: Add or remove up to 13 countries (`USA`, `BRA`, `DEU`, `GBR`, `JPN`, `CHN`, `FRA`, `CHE`, `CAN`, `AUS`, `MEX`, `ARG`, `IND`) directly in Tab `(d)` just like in `Currencies & DXY`.
2. **3-Tier Data Refresh & Permanent Local Storage (`SQLite WAL` + `is_real` Protection)**:
   - **Daily Market Data (10-min auto-refresh + `🔄 Refresh Now`)**: Automatically renews all 11 Global Stock Market Indices, active Stock OHLCV/multiples, and Currencies/DXY every 10 minutes, advancing `analysis_end_date` to today and using inclusive end dates (`end_date + 1 day`) in `yfinance`.
   - **Monthly Cycle (`cache_month = "YYYY-MM"`)**: Central Bank policy interest rates (including Brazil's **SELIC at 13.75% p.a.** and uniform 95-month trajectories across all countries from `2019-01` to the current month) and monthly macroeconomic indicators update on a monthly calendar cycle.
   - **Permanent Long-Term Database**: SQLite WAL mode with `is_real` protection guarantees synthetic fallback data never overwrites verified real data, and `🗑 Cache` clears only daily cache while preserving long-term macroeconomic and interest series.
3. **Reference Dates on All Cards & Smart Edge-Aware Chart Tooltips**:
   - Every index card, stock card, gainer/loser badge, currency card, and central bank rate card displays the exact **date corresponding to the data (`📅 YYYY-MM-DD` / `YYYY-MM`)**.
   - Chart tooltips appear **only when hovering directly over a variable's line/bar/candle**, displaying **only that variable's name, value, and date**, and automatically flip position near the chart's right, left, top, or bottom borders so they never get clipped.
   - Chart legends are positioned cleanly **below** every chart.
4. **Global Date Range, Dark/Light Theme & Offline Mode**:
   - Unified `📅 Global Date Range` bar (`Start`, `End`, `1M`, `6M`, `1Y`, `5Y`, `MAX`) synchronized across charts and tables.
   - Instant toggle between **🌙 Dark Mode** and **☀ Light Mode**, plus **📡 Offline Mode / 🌐 Online Mode**.

---

## 📁 1. Project Structure

```text
Economia/
├── Cargo.toml                  # Rust project configuration (v0.3.0)
├── requirements.txt            # Python data engine dependencies
├── DOCUMENTACAO_v0.3.md        # Complete Version 0.3 technical & functional documentation
├── README.md                   # General guide (Windows 11 & macOS)
├── Abrir_DAX_Economia.bat      # Fast 1-click launcher for Windows 11
├── instalar_e_executar.bat     # Full setup & launch script for Windows 11
├── gerar_pacote_portable.bat   # Builds Windows 11 Portable ZIP package
├── enviar_para_github.bat      # Commits, tags (v0.1, v0.2, v0.3) and pushes to GitHub
├── install_and_run_mac.sh      # 1-click setup & launch script for macOS
├── build_mac_app.sh            # Builds distributable macOS .app bundle and .zip
├── DAX_Economia.exe            # Active compiled Windows 11 binary (v0.3)
├── DAX_Economia_v0.3.exe       # Versioned Windows 11 binary (v0.3)
├── DAX_Economia_v0.2.exe       # Frozen Windows 11 binary (v0.2)
├── DAX_Economia_v0.1.exe       # Frozen Windows 11 binary (v0.1)
├── Versao_0.3/                 # Dedicated Version 0.3 archive (source, docs, and portable build)
├── config/
│   └── user_settings.json      # Automatic persistence of user preferences
├── data/
│   ├── cache.db                # Local SQLite WAL database with real-data protection
│   └── exports/                # Exported CSV tables, Plotly HTML reports & Matplotlib PNGs
├── python_engine/              # Data & Quantitative Analysis Layer (Python)
└── src/                        # Native GUI Presentation Layer (Rust + egui 0.29)
```

---

## 🪟 2. Running on Windows 11

- **Quick Launch**: Double-click **`Abrir_DAX_Economia.bat`** or **`DAX_Economia.exe`**.
- **First-Time Setup / Rebuild**: Double-click **`instalar_e_executar.bat`**.
- **Generate Portable ZIP**: Double-click **`gerar_pacote_portable.bat`**.
- **Push to GitHub**: Double-click **`enviar_para_github.bat`**.

## 🍎 3. Running on macOS

```bash
chmod +x install_and_run_mac.sh build_mac_app.sh
./install_and_run_mac.sh
```
To build the distributable `DAX Economy.app` and `dist/DAX_Economy_macOS.zip`:
```bash
./build_mac_app.sh
```

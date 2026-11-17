# 📊 DAX Economy — Cross-Platform Desktop Platform (Equities, FX DXY, Interest Rates & Macroeconomics)

Native desktop application for **macOS** and **Windows 11**, built with **Rust (`egui` / `eframe` / `egui_plot` / `egui_extras`)** for the visual interface and **Python (`pandas`, `numpy`, `yfinance`, `fredapi`, `pandas_datareader`, `requests`, `plotly`, `matplotlib`)** for the data and statistical analysis engine.

The visual design follows the reference layouts **`Dax1.png` through `Dax4.png`**:
- **`Dax1.png`**: Permanent top bar with mini-cards and *sparklines* (`IBOV`, `DOLFUT`, `DOW`, `DXY=`, `PETR4`, `VALE3`, `CDI/SELIC`, `IPCA`) + **▲ Top Gainers** (green badges) and **▼ Top Losers** (red badges).
- **`Dax2.png` (TradeMap Style)**: Dark sidebar, quick search, favorites, and **Asset Cards** grid with individual trend sparklines, Market Cap (*Mcap*), *Dividend Yield*, and 12-month return.
- **`Dax3.png` (StatusInvest Style)**: Detailed stock panel with **Current Price** highlighted in blue, **52-Week Low/High**, **Month Low/High**, **Dividend Yield (12M)**, **Return (12M and Current Month)**, period selector (`5 DAYS` to `5 YEARS`), real-time currency converter (`R$ BRL`, `US$ USD`, `€ EUR`), and interactive **Candlesticks (OHLC)** or **Continuous Line** chart with Moving Averages (`SMA 20` and `SMA 50`).
- **`Dax4.png` (xStation Scanner Style)**: **Stocks Scanner** with sliders to filter companies by Market Cap, P/E Ratio, *Dividend Yield*, ROE, and Country, displayed in an interactive `egui_extras` table.

---

## 📁 1. Project Structure

```text
Economy/
├── Cargo.toml                  # Rust project configuration and dependencies (egui, eframe, egui_plot, egui_extras)
├── requirements.txt            # Python data engine dependencies (pandas, yfinance, fredapi, plotly, etc.)
├── .env.example                # Optional FRED API key template (FRED_API_KEY)
├── install_and_run_mac.sh      # 1-click setup & launch script for macOS
├── build_mac_app.sh            # Builds distributable macOS .app bundle and .zip package (dist/DAX_Economy_macOS.zip)
├── instalar_e_executar.bat     # 1-click setup & launch script for Windows 11 (creates .venv, installs packages, opens app)
├── executar_motor_teste.bat    # Script to test Python engine modules in Windows terminal
├── README.md                   # User & developer guide (macOS & Windows 11)
├── config/
│   └── user_settings.json      # Automatic persistence of user preferences (favorites, countries, currency)
├── data/
│   ├── cache.db                # SQLite database created automatically for fast local caching
│   └── exports/                # Exported interactive HTML charts (Plotly) and PNG images (Matplotlib)
├── python_engine/              # Data & Analysis Layer (Python)
│   ├── __init__.py
│   ├── cli.py                  # CLI entrypoint called by Rust via subprocess (UTF-8 JSON)
│   ├── cache_manager.py        # Local SQLite cache manager with TTL expiration
│   ├── stocks_service.py       # Stock quotes (yfinance), OHLCV, moving averages, Gainers/Losers, and Scanner
│   ├── forex_service.py        # Currency quotes, US Dollar Index (DXY=), and FX converter
│   ├── interest_service.py     # Central Bank interest rates (Fed, Selic BCB, ECB, BoE, BoJ) & Real Rate
│   ├── macro_service.py        # Socioeconomic data from World Bank API (GDP, GDP per capita, CPI, Unemployment, etc.)
│   └── chart_exporter.py       # Chart export in Plotly (HTML) and Matplotlib (PNG)
└── src/                        # Presentation & UI Logic Layer (Rust + egui)
    ├── main.rs                 # Native desktop window entrypoint (macOS & Windows 11)
    ├── models.rs               # Strongly typed data structures (Serde JSON)
    ├── config.rs               # Automatic read/write of config/user_settings.json
    ├── bridge.rs               # Non-blocking async Rust <-> Python bridge (threads + mpsc channels)
    ├── theme.rs                # Dark Mode financial theme, green/red badges, and Sparklines renderer
    └── app.rs                  # Navigation and views for all 5 tabs: (a), (b), (c), (d), and (e)
```

---

## 🍎 2. Installation & Running on macOS

### Option A: Ready-to-Use Application (End Users)
1. Download and extract **`DAX_Economy_macOS.zip`** (built via `./build_mac_app.sh` into `dist/`).
2. Drag **`DAX Economy.app`** to your **Applications** folder (or run it directly).
3. Ensure Python 3.10+ is installed on your Mac. On first launch, the app automatically sets up an isolated Python environment in `~/Library/Application Support/DAX_Economy/.venv`.
   > **Note (Gatekeeper):** If macOS says the app is from an unidentified developer, right-click `DAX Economy.app` → **Open** → **Open**.

### Option B: Running from Source (Developers)
1. **Install Python 3.10+**: Download from [python.org](https://www.python.org/downloads/) or via Homebrew (`brew install python`).
2. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. **Run with 1 command**:
   ```bash
   chmod +x install_and_run_mac.sh build_mac_app.sh
   ./install_and_run_mac.sh
   ```
4. **Build distributable `.app` and `.zip`**:
   ```bash
   ./build_mac_app.sh
   ```
   The distributable package will be generated at `dist/DAX_Economy_macOS.zip`.

---

## 🪟 3. Installation & Running on Windows 11

You don't need to know how to program. Follow these **3 simple steps** once:

### Step 1: Install Python (Data Engine)
1. Visit: [https://www.python.org/downloads/](https://www.python.org/downloads/)
2. Click **"Download Python 3.12+"**.
3. Run the installer and **CHECK `"Add python.exe to PATH"`** at the bottom of the first screen before clicking **"Install Now"**.

### Step 2: Install Rust (Native Desktop UI)
1. On Windows 11, the Rust compiler uses Microsoft C++ Build Tools. If not installed yet, download [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/pt-br/visual-cpp-build-tools/) (select *"Desktop development with C++"*).
2. Next, visit [https://rustup.rs/](https://rustup.rs/) and download **`rustup-init.exe (64-bit)`**.
3. Press **`1`** and **`Enter`** for the default installation.

### Step 3: Launch with Double-Click
1. Open the project folder.
2. Double-click **`instalar_e_executar.bat`**.
3. The script will automatically create `.venv`, install Python libraries, compile the app (`cargo run --release`), and launch the window!

> **Optional Tip (FRED API Key):** The app works **100% out of the box without any API key**, querying open endpoints from the Central Bank of Brazil, FRED, and the World Bank, with local SQLite caching. If you want to use your own `fredapi` key, copy `.env.example` to `.env` and paste your free key from `https://fred.stlouisfed.org/docs/api/api_key.html`.

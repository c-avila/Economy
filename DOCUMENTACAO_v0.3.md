# 📘 Documentação Oficial — DAX - Data Analysis Exchange (Versão 0.3)

**Plataforma Desktop Multiplataforma (Windows 11 & macOS) para Análise do Mercado de Ações, Câmbio (`DXY`), Taxas de Juros Globais e Indicadores Macroeconômicos de Ciclo de Mercado.**

---

## 1. Visão Geral da Arquitetura (Versão 0.3)

O **DAX - Data Analysis Exchange (v0.3)** combina uma interface gráfica nativa compilada em **Rust (`egui 0.29` / `eframe` / `egui_plot` / `egui_extras`)** com um motor quantitativo e estatístico em **Python (`pandas`, `numpy`, `yfinance`, `fredapi`, `pandas_datareader`, `requests`, `plotly`, `matplotlib`, `sqlite3`)**.

### Camadas do Sistema
1. **Camada de Apresentação e Interação (`src/` — Rust + `egui 0.29`)**:
   - Renderização nativa acelerada por GPU a 60 FPS sem bloqueio de interface (*non-blocking UI*).
   - Suporte dinâmico a **Tema Escuro (*Dark Mode*)** e **Tema Claro (*Light Mode*)** ([`src/theme.rs`](src/theme.rs)).
   - Comunicação assíncrona via threads e canais `std::sync::mpsc` ([`src/bridge.rs`](src/bridge.rs)).
   - Gráficos interativos com **legendas posicionadas abaixo da área de plotagem** (evitando qualquer sobreposição com curvas ou barras) e **tooltips inteligentes sensíveis à linha e às bordas** (exibidos apenas quando o cursor está sobre a curva da variável, mostrando exclusivamente o nome da variável, seu valor e a data correspondente, com inversão automática de posição próximo às bordas direita, esquerda, superior e inferior).
2. **Camada de Coleta, Validação e Análise (`python_engine/` — Python 3.10+)**:
   - Coleta paralela (*multi-threading* com `ThreadPoolExecutor`) em APIs financeiras e macroeconômicas oficiais:
     - **Yahoo Finance (`yfinance`)**: Índices globais, ações B3/NYSE/Nasdaq/Europa, OHLCV, médias móveis (`SMA 20`, `SMA 50`), múltiplos fundamentalistas e pares cambiais + Índice Dólar (`DX-Y.NYB` / `DXY=`).
     - **Federal Reserve Economic Data (`FRED`) & Banco Central do Brasil (`BCB SGS 432`)**: Séries históricas mensais de taxas básicas de juros de 12 Bancos Centrais (`US`, `BR`, `EU`, `GB`, `JP`, `CN`, `CH`, `CA`, `AU`, `MX`, `AR`, `IN`), cálculo de inflação CPI e Taxa de Juro Real Ex-Post (Equação de Fisher).
     - **World Bank REST API v2 (WDI) + Séries Oficiais FRED / OECD / BLS / U.S. Census / S&P Global PMI**: 14 indicadores macroeconômicos e de ciclo de mercado classificados em **Leading**, **Coincident** e **Lagging Indicators**.
3. **Camada de Persistência Local e Integridade (`data/cache.db` + `config/user_settings.json`)**:
   - Banco de dados **SQLite em modo `WAL` (*Write-Ahead Logging*)** com `busy_timeout=20000` e proteção `is_real` ([`python_engine/cache_manager.py`](python_engine/cache_manager.py)): dados sintéticos de contingência (`is_real=0`) **nunca sobrescrevem** cotações ou séries reais (`is_real=1`) já salvas.

---

## 2. Política de Atualização e Armazenamento em 3 Níveis

A **Versão 0.3** separa rigorosamente os dados de mercado que variam ao longo do pregão diário dos dados estruturais e macroeconômicos de longo prazo:

| Categoria de Dado | Ciclo de Renovação | Mecanismo de Armazenamento e Atualização |
| :--- | :--- | :--- |
| **1. Dados Diários de Mercado**<br>*(11 Bolsas Mundiais, Ações OHLCV, Altas/Baixas, Scanner, Moedas e DXY)* | **Constante / Diário**<br>*(Auto-refresh a cada **10 minutos** + botão `🔄 Refresh Now`)* | • TTL de 10 minutos (`600s`) no SQLite.<br>• Avanço automático de `analysis_end_date` para a data atual (`YYYY-MM-DD`) e `end_date + 1 dia` no `yfinance` para incluir o pregão do próprio dia.<br>• Recuperação automática do último dado real verificado caso a conexão oscile. |
| **2. Dados Macroeconômicos Mensais e Taxas de Juros**<br>*(Juros dos BCs, Curva de Juros 10Y-2Y, CPI, Desemprego, NFP, Varejo, Produção Industrial, Imobiliário, PMI, Câmbio Oficial)* | **Mensal**<br>*(Controle por mês calendário `cache_month = "YYYY-MM"`)* | • Armazenados permanentemente por país e por indicador (`interest_country_real_v7_*` e `macro_ind_real_v6_*`).<br>• Dentro do mesmo mês, carregam instantaneamente do banco local sem atrasar a abertura.<br>• Na virada do mês (ou via botões `🔄 Update Rates (Monthly)` / `🔄 Update Macro Data (Online)`), sincronizam os novos dados do mês atual. |
| **3. Dados Estruturais de Longo Prazo**<br>*(PIB Nominal Total, Crescimento Real do PIB, PIB per Capita e População)* | **Trimestral / Anual**<br>*(Persistência permanente local)* | • Gravados permanentemente em `data/cache.db` e no snapshot instantâneo `data/macro_permanent.json` (carregamento em `0 ms` na inicialização).<br>• Preservados intactos mesmo ao limpar o cache diário pelo botão `🗑 Cache`. |

---

## 3. Detalhamento das Janelas e Abas da Aplicação

### 🌐 Barra Superior Permanente: Índices das 11 Principais Bolsas Mundiais + `Global Date Range`
- **11 Bolsas Mundiais**: `IBOV` (Brasil B3), `WALL ST (DOW)` (NYSE), `S&P 500` (EUA), `NASDAQ` (EUA Tech), `NIKKEI 225` (Japão), `XANGAI (SSE)` (China), `DAX 40` (Alemanha), `FTSE 100` (Reino Unido), `CAC 40` (França), `IBEX 35` (Espanha) e `MERVAL` (Argentina).
- Cada mini-card exibe: **Pontuação atual**, **Variação percentual do dia**, **Data correspondente ao dado (`País • YYYY-MM-DD`)** e **Sparkline** dos últimos pregões (clicável para abrir o gráfico completo do índice).
- **Seletor `📅 Global Date Range`**: Permite definir `Start` (`YYYY-MM-DD`), `End` (`YYYY-MM-DD`) ou presets rápidos (`1M`, `6M`, `1Y`, `5Y`, `MAX`), aplicando o intervalo em todos os gráficos e tabelas históricas do sistema.

---

### ⚡ Aba `(e) Dashboard & Scanner` (Estilos `Dax1`, `Dax2` e `Dax4`)
1. **Painéis Independentes `⬆ Top Gainers` e `⬇ Top Losers`**:
   - Organizados em 2 cards independentes lado a lado com grade estruturada de 4 colunas (`egui::Grid`), exibindo a data do pregão (`📅 YYYY-MM-DD`) e botões de variação percentual sem risco de sobreposição.
2. **Modo `📊 Cards with Sparklines (Dax2 / TradeMap Style)`**:
   - Grade de 3 colunas com código do ativo, nome da empresa, **data da cotação (`📅 YYYY-MM-DD`)**, preço na moeda nativa, badge de variação diária, gráfico *sparkline*, *Market Cap*, *Dividend Yield* e valorização em 12 meses.
3. **Modo `🔍 Stocks Scanner & Filters (Dax4 / xStation Style)`**:
   - Filtros em tempo real por **País**, **Busca por texto (Ticker/Nome/Setor)** e *sliders* para **Capitalização Mínima (Bi)**, **P/E Máximo (P/L)**, **Dividend Yield Mínimo (%)** e **ROE Mínimo (%)**.

---

### 📈 Aba `(a) Equities & Candles` (Estilo `Dax3 / StatusInvest`)
- Consulta de qualquer ativo nacional ou internacional (ex.: `PETR4`, `VALE3`, `ITUB4`, `WEGE3`, `AAPL`, `NVDA`, `MSFT`, `AMZN`, `SAP`, `ASML`), com adição/remoção rápida de **Ativos Favoritos** na barra lateral esquerda.
- **Conversão Cambial Automática**: Visualização de preços e múltiplos na **Moeda Nativa**, **`R$ (BRL)`**, **`US$ (USD)`** ou **`€ (EUR)`**.
- **5 Cards Superiores**:
  1. `CURRENT PRICE (YYYY-MM-DD)` com variação diária e data do pregão;
  2. `52-WEEK LOW` & `MONTH LOW`;
  3. `52-WEEK HIGH` & `MONTH HIGH`;
  4. `DIVIDEND YIELD` & dividendos pagos nos últimos 12 meses;
  5. `RETURN (12M)` & variação no mês atual.
- **Gráfico Principal Interativo**:
  - Alternância entre **`🕯 Candlesticks (OHLC)`** e **`📈 Continuous Line`**, com médias móveis **`SMA 20`** e **`SMA 50`**.
  - **Legenda abaixo do gráfico** e **tooltip inteligente sobre a linha/candle** com reposicionamento automático nas bordas.
  - Exportação direta para **Relatório HTML Interativo (Plotly)**, **Imagem PNG (Matplotlib)** e **Tabela Histórica OHLCV (CSV)**.

---

### 💵 Aba `(b) Currencies & DXY`
- **Conversor Cambial em Tempo Real** entre 12 moedas (`USD`, `BRL`, `EUR`, `GBP`, `JPY`, `CNY`, `CHF`, `CAD`, `AUD`, `MXN`, `ARS`, `INR`).
- **Gerenciador de Carteira Cambial (Até 10 Moedas + `DXY=`)**: Inserção (`➕ Add Currency`) e retirada (`✖`) dinâmica de moedas.
- **Destaques vs. Dólar (`Spotlights vs USD`)**: Escolha de até **3 moedas em destaque** lado a lado com o card do **Índice Dólar (`DXY=`)**, exibindo cotação atual, *sparkline*, variação no período e **data da cotação (`📅 YYYY-MM-DD`)**.
- **Gráfico Comparativo Cambial (Base 100 ou Cotação Nominal)**:
  - Checkboxes individuais (`Show/Hide on chart`, `Select All`, `Deselect All`) para ativar/desativar curvas no gráfico.
  - Tooltip exclusivo sobre a linha da moeda apontada pelo mouse e legenda posicionada abaixo do gráfico.

---

### 🏦 Aba `(c) Interest Rates (CBs)`
- **Ciclo de Atualização Mensal (`📅 Update Cycle: Monthly`)** sincronizado com os países das moedas selecionadas na Aba `(b)`.
- **Grade de Cards dos Bancos Centrais** (3 colunas): Destaque azul para o **Federal Reserve (`US` — `4,33% a.a.`)** e cards para o **Banco Central do Brasil (`BR` — `SELIC 13,75% a.a.`)**, **BCE (`EU` — `2,90%`)**, **Bank of England (`GB` — `4,50%`)**, **Bank of Japan (`JP` — `0,50%`)**, **PBoC (`CN` — `3,00%`)**, **SNB (`CH`)**, **BoC (`CA`)**, **RBA (`AU`)**, **Banxico (`MX`)**, **BCRA (`AR`)** e **RBI (`IN`)**, exibindo **código da série**, **data de referência (`📅 2026-11`)**, **Inflação CPI** e **Juro Real Ex-Post**.
- **Gráfico da Trajetória Histórica das Taxas de Juros (`% p.a.`)**:
  - Todas as séries são normalizadas em uma grade mensal contínua e uniforme (`2019-01` até o mês atual `2026-11`), garantindo alinhamento temporal perfeito entre todos os países no eixo X.

---

### 🌍 Aba `(d) Macroeconomics (WB & Global Cycles)`
- **Seleção Independente de Países (Até 13 Economias)**:
  - Permite adicionar (`➕ Add Country`) e remover (`✖`) livremente quaisquer países entre `USA`, `BRA`, `DEU`, `GBR`, `JPN`, `CHN`, `FRA`, `CHE`, `CAN`, `AUS`, `MEX`, `ARG` e `IND` (com botão opcional `🔗 Sync with (b) Currencies`).
- **14 Indicadores Macroeconômicos e de Ciclo de Mercado**:
  1. **Total Nominal GDP (US$ Billions)** — *Lagging* | *Quarterly / Annual* | Fonte: World Bank WDI (`NY.GDP.MKTP.CD`)
  2. **Real GDP Growth (% YoY)** — *Lagging* | *Quarterly / Annual* | Fonte: World Bank WDI (`NY.GDP.MKTP.KD.ZG`)
  3. **GDP per Capita (Current US$)** — *Lagging* | *Annual* | Fonte: World Bank WDI (`NY.GDP.PCAP.CD`)
  4. **Consumer Price Index — CPI Inflation (% YoY)** — *Lagging* | *Monthly* | Fonte: World Bank WDI (`FP.CPI.TOTL.ZG`) / BLS / IBGE
  5. **Policy Interest Rate (% p.a.)** — *Leading / Lagging* | *Monthly* | Fonte: FRED (`FEDFUNDS`) / BCB (`Selic 13,75%`) / ECB / BIS
  6. **Yield Curve Spread (10Y - 2Y Sovereign, % pts)** — *Leading* | *Monthly* | Fonte: U.S. Treasury / FRED (`T10Y2Y`) / OECD
  7. **Employment: NFP / Payroll Growth (% YoY & Jobless Claims)** — *Coincident / Leading* | *Monthly* | Fonte: U.S. BLS / FRED (`PAYEMS` & `ICSA`)
  8. **Unemployment Rate (% of Labor Force)** — *Lagging* | *Monthly* | Fonte: World Bank WDI (`SL.UEM.TOTL.ZS`) / BLS / IBGE
  9. **Retail Sales & Consumption Growth (% YoY)** — *Coincident* | *Monthly* | Fonte: U.S. Census / FRED (`RSAFS`) / World Bank / IBGE (`PMC`)
  10. **Industrial Production & Inventories (% YoY)** — *Coincident* | *Monthly* | Fonte: Federal Reserve / FRED (`INDPRO` & `BUSINV`) / IBGE (`PIM-PF`)
  11. **Real Estate: Building Permits & Home Sales (% YoY)** — *Leading* | *Monthly* | Fonte: U.S. Census / FRED (`PERMIT` & `HOUST`) / OECD
  12. **PMI — Purchasing Managers' Index (50+ = Expansion)** — *Leading* | *Monthly* | Fonte: S&P Global / ISM Manufacturing & Services PMI
  13. **Official Exchange Rate (LCU / US$)** — *Coincident / Leading* | *Monthly* | Fonte: World Bank WDI (`PA.NUS.FCRF`)
  14. **Total Population (Millions)** — *Lagging / Structural* | *Annual* | Fonte: World Bank WDI (`SP.POP.TOTL`)
- **Card de Referência do Indicador Ativo + Guia Analítico (*Leading vs. Coincident vs. Lagging*) + Diretório de Fontes Oficiais**:
  - Exibe o que cada indicador mede, quais setores da bolsa são impactados, a periodicidade de atualização e links clicáveis (`🔗 Open Official Data Source & Methodology` / `🔗 Source`) para abrir a documentação oficial no navegador.

---

## 4. Estrutura do Diretório Dedicado `Versao_0.3/`

```text
Versao_0.3/
├── DOCUMENTACAO_v0.3.md                        # Documentação completa da Versão 0.3
├── README.md                                   # Guia rápido multiplataforma (Windows 11 & macOS)
├── Cargo.toml                                  # Manifesto Rust (v0.3.0)
├── Cargo.lock                                  # Lockfile de dependências Rust
├── requirements.txt                            # Dependências do motor Python
├── .env.example                                # Modelo de configuração opcional FRED_API_KEY
├── Abrir_DAX_Economia.bat                      # Inicializador instantâneo Windows 11
├── instalar_e_executar.bat                     # Instalador e inicializador completo Windows 11
├── gerar_pacote_portable.bat                   # Gerador de distribuição Portable (.ZIP)
├── enviar_para_github.bat                      # Script de versionamento e push para o GitHub
├── install_and_run_mac.sh                      # Inicializador para macOS
├── build_mac_app.sh                            # Construtor de pacote .app para macOS
├── DAX_Economia_v0.3.exe                       # Executável nativo Windows 11 compilado (v0.3)
├── config/
│   └── user_settings.json                      # Configurações padrão persistidas em JSON
├── python_engine/                              # Código-fonte completo do motor Python (v0.3.0)
│   ├── __init__.py
│   ├── cache_manager.py
│   ├── chart_exporter.py
│   ├── cli.py
│   ├── forex_service.py
│   ├── interest_service.py
│   ├── macro_service.py
│   └── stocks_service.py
├── src/                                        # Código-fonte completo da interface Rust (v0.3.0)
│   ├── app.rs
│   ├── bridge.rs
│   ├── config.rs
│   ├── main.rs
│   ├── models.rs
│   └── theme.rs
└── Portable/                                   # Distribuição Portável pronta para uso
    ├── DAX_Economia_v0.3_Portable_Win11.zip    # Pacote ZIP Portável da Versão 0.3
    └── DAX_Economia_Portable/                  # Pasta Portável descompactada
```


# 📊 DAX Economia — Plataforma Desktop Windows 11 (Ações, Câmbio DXY, Juros e Macroeconomia)

Aplicação desktop nativa para **Windows 11** construída com **Rust (`egui` / `eframe` / `egui_plot` / `egui_extras`)** na camada visual e **Python (`pandas`, `numpy`, `yfinance`, `fredapi`, `pandas_datareader`, `requests`, `plotly`, `matplotlib`)** no motor de dados e análise estatística.

O design visual foi construído seguindo diretamente as referências **`Dax1.png` a `Dax4.png`**:
- **`Dax1.png`**: Barra superior permanente com mini-cards e *sparklines* (`IBOV`, `DOLFUT`, `DOW`, `DXY=`, `PETR4`, `VALE3`, `CDI/SELIC`, `IPCA`) + painel de **▲ Altas** (badges verdes) e **▼ Baixas** (badges vermelhos).
- **`Dax2.png` (Estilo TradeMap)**: Menu lateral escuro, busca rápida, favoritos e grade de **Cards de Ativos** com gráfico de tendência individual, Capitalização (*Mcap*), *Dividend Yield* e variação em 12 meses.
- **`Dax3.png` (Estilo StatusInvest)**: Painel detalhado por ação com **Valor Atual** em destaque azul, **Mín/Máx 52 Semanas**, **Mín/Máx no Mês**, **Dividend Yield (12M)**, **Valorização (12M e Mês Atual)**, seletor de período (`5 DIAS` a `5 ANOS`), conversor de moeda em tempo real (`R$ BRL`, `US$ USD`, `€ EUR`) e gráfico interativo de **Candlesticks (OHLC)** ou **Linha Contínua** com Médias Móveis (`SMA 20` e `SMA 50`).
- **`Dax4.png` (Estilo xStation Scanner)**: **Stocks Scanner** com barras deslizantes (*sliders*) para filtrar empresas por Capitalização de Mercado, P/L (*P/E*), *Dividend Yield*, ROE e País, exibidos em tabela interativa `egui_extras`.

---

## 📁 1. Estrutura Completa do Projeto

```text
E:\AI\Economia\
├── Cargo.toml                  # Dependências e configuração do projeto Rust (egui, eframe, egui_plot, egui_extras)
├── requirements.txt            # Dependências do motor de dados em Python (pandas, yfinance, fredapi, plotly, etc.)
├── .env.example                # Modelo para chave opcional da API do FRED (FRED_API_KEY)
├── instalar_e_executar.bat     # Inicializador de 1 clique para Windows 11 (cria .venv, instala pacotes e abre o app)
├── executar_motor_teste.bat    # Script para testar os 4 módulos do motor Python no terminal
├── README.md                   # Este guia passo a passo para iniciantes no Windows 11
├── config/
│   └── user_settings.json      # Persistência automática das preferências do usuário (favoritos, países, moeda)
├── data/
│   ├── cache.db                # Banco SQLite criado automaticamente para cache local de alta velocidade
│   └── exports/                # Gráficos interativos HTML (Plotly) e imagens PNG (Matplotlib) exportados
├── python_engine/              # Camada de Dados e Análise (Python)
│   ├── __init__.py
│   ├── cli.py                  # Ponto de entrada CLI chamado pelo Rust via subprocess (JSON em UTF-8)
│   ├── cache_manager.py        # Gerenciador de cache local em SQLite com controle de expiração (TTL)
│   ├── stocks_service.py       # Coleta de ações (yfinance), OHLCV, médias móveis, Altas/Baixas e Scanner
│   ├── forex_service.py        # Cotações de moedas, Índice Dólar (DXY=) e conversor cambial
│   ├── interest_service.py     # Taxas de juros (Fed, Selic BCB, BCE, BoE, BoJ) e cálculo de Juro Real
│   ├── macro_service.py        # Dados socioeconômicos da API do World Bank (PIB, PIB per capita, CPI, Desemprego, etc.)
│   └── chart_exporter.py       # Exportação de relatórios gráficos em Plotly (HTML) e Matplotlib (PNG)
└── src/                        # Camada de Apresentação e Lógica (Rust + egui)
    ├── main.rs                 # Inicialização da janela nativa Windows 11
    ├── models.rs               # Estruturas de dados fortemente tipadas (Serde JSON)
    ├── config.rs               # Leitura e gravação automática de config/user_settings.json
    ├── bridge.rs               # Ponte assíncrona não-bloqueante Rust ↔ Python (threads + mpsc channels)
    ├── theme.rs                # Tema visual Dark Mode, badges verde/vermelho e desenhador de Sparklines
    └── app.rs                  # Navegação e telas das 5 abas: (a), (b), (c), (d) e (e)
```

---

## 🛠️ 2. Passo a Passo de Instalação para Iniciantes no Windows 11

Você não precisa saber programar. Siga apenas os **3 passos simples** abaixo para preparar seu computador uma única vez:

### Passo 1: Instalar o Python (Motor de Dados)
1. Acesse o site oficial: [https://www.python.org/downloads/](https://www.python.org/downloads/)
2. Clique no botão amarelo **"Download Python 3.12+"**.
3. Abra o instalador baixado e **MARQUE A CAIXINHA `"Add python.exe to PATH"`** na parte inferior da primeira tela antes de clicar em **"Install Now"**.

### Passo 2: Instalar o Rust (Motor Gráfico Nativo Windows 11)
1. No Windows 11, o compilador Rust utiliza as ferramentas C++ da Microsoft. Caso ainda não tenha, baixe e instale o [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/pt-br/visual-cpp-build-tools/) (marque a opção *"Desenvolvimento para desktop com C++"*).
2. Em seguida, acesse [https://rustup.rs/](https://rustup.rs/) e clique em **`rustup-init.exe (64-bit)`**.
3. Ao abrir a janela preta do instalador, pressione **`1`** e aperte **`Enter`** para a instalação padrão.

### Passo 3: Executar a Plataforma com Dois Cliques!
1. Abra a pasta `E:\AI\Economia`.
2. Dê **dois cliques** no arquivo **`instalar_e_executar.bat`**.
3. O script fará tudo sozinho:
   - Criará o ambiente isolado `.venv` do Python;
   - Instalará todas as bibliotecas (`yfinance`, `pandas`, `fredapi`, `plotly`, `matplotlib`, etc.);
   - Compilará o aplicativo em modo de máxima velocidade (`cargo run --release`) e abrirá a janela no seu Windows 11!

> **Dica Opcional (Chave do FRED):** O aplicativo já funciona **100% sem precisar de chave**, pois consulta diretamente a API aberta do Banco Central do Brasil, os dados abertos do FRED e a API do Banco Mundial, além de possuir cache em SQLite. Se desejar configurar uma chave própria do `fredapi`, basta copiar o arquivo `.env.example` para `.env` e colar sua chave gratuita obtida em `https://fred.stlouisfed.org/docs/api/api_key.html`.


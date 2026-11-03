//! Ponte de comunicação assíncrona Rust ↔ Python via `subprocess` e `mpsc::channel`.
//! Garante que nenhuma requisição de rede ou análise de dados trave a interface gráfica `egui`.

use crate::models::{
    ChartExportResponse, ForexResponse, InterestResponse, MacroResponse, MarketOverviewResponse,
    StockDetailResponse,
};
use eframe::egui;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Mensagens retornadas assincronamente pelas threads de trabalho Python para a GUI Rust.
#[derive(Debug)]
pub enum BridgeMessage {
    OverviewLoaded(MarketOverviewResponse),
    StockLoaded(StockDetailResponse),
    ForexLoaded(ForexResponse),
    InterestLoaded(InterestResponse),
    MacroLoaded(MacroResponse),
    ChartExported(ChartExportResponse),
    Error(String),
}

/// Gerenciador de chamadas assíncronas para o motor de dados em Python.
pub struct PythonBridge {
    tx: Sender<BridgeMessage>,
    rx: Receiver<BridgeMessage>,
    pub active_tasks: usize,
}

impl Default for PythonBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonBridge {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            active_tasks: 0,
        }
    }

    /// Coleta todas as mensagens prontas na fila sem bloquear o frame da interface.
    pub fn poll_messages(&mut self) -> Vec<BridgeMessage> {
        let mut msgs = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            if self.active_tasks > 0 {
                self.active_tasks -= 1;
            }
            msgs.push(msg);
        }
        msgs
    }

    /// Detecta o executável Python prioritariamente no ambiente isolado `.venv` do Windows 11.
    fn detectar_executavel_python() -> String {
        let candidatos = [
            r".venv\Scripts\python.exe",
            r"venv\Scripts\python.exe",
            ".venv/bin/python",
        ];
        for caminho in candidatos {
            if Path::new(caminho).exists() {
                return caminho.to_string();
            }
        }
        "python".to_string()
    }

    /// Executa `python -m python_engine.cli ...` e retorna a saída JSON bruta.
    fn executar_cli_python(args: &[String]) -> Result<String, String> {
        let python_bin = Self::detectar_executavel_python();
        let mut cmd = Command::new(&python_bin);
        cmd.arg("-m").arg("python_engine.cli").args(args);
        cmd.env("PYTHONIOENCODING", "utf-8");

        // No Windows, evita abrir janela preta de console ao disparar subprocesso
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output().map_err(|e| {
            format!(
                "Não foi possível executar o motor Python ('{python_bin}'). Verifique se o Python e a .venv estão instalados: {e}"
            )
        })?;

        let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success() && stdout_str.is_empty() {
            return Err(format!(
                "O motor Python retornou erro (código {:?}): {}",
                output.status.code(),
                if stderr_str.is_empty() {
                    "Sem detalhes adicionais."
                } else {
                    &stderr_str
                }
            ));
        }

        if stdout_str.is_empty() {
            return Err("O motor Python retornou resposta vazia.".to_string());
        }

        // Caso alguma biblioteca imprima avisos antes do JSON, extrai a última linha JSON válida
        let json_line = stdout_str
            .lines()
            .rev()
            .find(|l| l.trim_start().starts_with('{'))
            .unwrap_or(&stdout_str)
            .to_string();

        Ok(json_line)
    }

    /// Solicita o carregamento do Dashboard Geral e Scanner (Dax1, Dax2, Dax4).
    pub fn request_overview(&mut self, favoritos: &[String], force_refresh: bool, ctx: egui::Context) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let tickers_arg = favoritos.join(",");

        thread::spawn(move || {
            let mut args = vec![
                "overview".to_string(),
                "--tickers".to_string(),
                tickers_arg,
            ];
            if force_refresh {
                args.push("--no-cache".to_string());
            }

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<MarketOverviewResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::OverviewLoaded(resp),
                    Ok(_) => BridgeMessage::Error(format!("Resposta inesperada no Overview: {json_str}")),
                    Err(e) => BridgeMessage::Error(format!("Erro ao interpretar JSON do Overview: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Solicita análise detalhada OHLCV e múltiplos de uma ação (Dax3).
    pub fn request_stock(
        &mut self,
        ticker: &str,
        period: &str,
        currency: &str,
        force_refresh: bool,
        ctx: egui::Context,
    ) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let ticker_s = ticker.trim().to_uppercase();
        let period_s = period.to_string();
        let curr_s = currency.to_string();

        thread::spawn(move || {
            let mut args = vec![
                "stock".to_string(),
                "--ticker".to_string(),
                ticker_s,
                "--period".to_string(),
                period_s,
                "--currency".to_string(),
                curr_s,
            ];
            if force_refresh {
                args.push("--no-cache".to_string());
            }

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<StockDetailResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::StockLoaded(resp),
                    Ok(_) => BridgeMessage::Error(format!("Erro ao consultar ação: {json_str}")),
                    Err(e) => BridgeMessage::Error(format!("Erro ao ler dados da ação: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Solicita cotações de moedas e Índice Dólar (DXY=).
    pub fn request_forex(
        &mut self,
        pairs: &[String],
        period: &str,
        force_refresh: bool,
        ctx: egui::Context,
    ) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let pairs_arg = pairs.join(",");
        let period_s = period.to_string();

        thread::spawn(move || {
            let mut args = vec![
                "forex".to_string(),
                "--pairs".to_string(),
                pairs_arg,
                "--period".to_string(),
                period_s,
            ];
            if force_refresh {
                args.push("--no-cache".to_string());
            }

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<ForexResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::ForexLoaded(resp),
                    Ok(_) => BridgeMessage::Error(format!("Erro na consulta de moedas: {json_str}")),
                    Err(e) => BridgeMessage::Error(format!("Erro ao processar JSON de moedas: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Solicita taxas de juros dos Bancos Centrais selecionados.
    pub fn request_interest(
        &mut self,
        countries: &[String],
        start_year: i32,
        force_refresh: bool,
        ctx: egui::Context,
    ) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let countries_arg = countries.join(",");

        thread::spawn(move || {
            let mut args = vec![
                "interest".to_string(),
                "--countries".to_string(),
                countries_arg,
                "--start-year".to_string(),
                start_year.to_string(),
            ];
            if force_refresh {
                args.push("--no-cache".to_string());
            }

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<InterestResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::InterestLoaded(resp),
                    Ok(_) => BridgeMessage::Error(format!("Erro na consulta de juros: {json_str}")),
                    Err(e) => BridgeMessage::Error(format!("Erro ao ler JSON de juros: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Solicita variáveis socioeconômicas do World Bank API.
    pub fn request_macro(
        &mut self,
        countries: &[String],
        start_year: i32,
        end_year: i32,
        force_refresh: bool,
        ctx: egui::Context,
    ) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let countries_arg = countries.join(",");

        thread::spawn(move || {
            let mut args = vec![
                "macro".to_string(),
                "--countries".to_string(),
                countries_arg,
                "--start-year".to_string(),
                start_year.to_string(),
                "--end-year".to_string(),
                end_year.to_string(),
            ];
            if force_refresh {
                args.push("--no-cache".to_string());
            }

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<MacroResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::MacroLoaded(resp),
                    Ok(_) => BridgeMessage::Error(format!("Erro na consulta World Bank: {json_str}")),
                    Err(e) => BridgeMessage::Error(format!("Erro ao ler JSON macroeconômico: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    /// Solicita exportação de gráfico interativo Plotly (HTML) ou Matplotlib (PNG).
    pub fn request_export_chart(
        &mut self,
        ticker: &str,
        period: &str,
        currency: &str,
        format: &str,
        ctx: egui::Context,
    ) {
        self.active_tasks += 1;
        let tx = self.tx.clone();
        let ticker_s = ticker.to_string();
        let period_s = period.to_string();
        let curr_s = currency.to_string();
        let fmt_s = format.to_string();

        thread::spawn(move || {
            let args = vec![
                "export_chart".to_string(),
                "--ticker".to_string(),
                ticker_s,
                "--period".to_string(),
                period_s,
                "--currency".to_string(),
                curr_s,
                "--format".to_string(),
                fmt_s,
            ];

            let msg = match Self::executar_cli_python(&args) {
                Ok(json_str) => match serde_json::from_str::<ChartExportResponse>(&json_str) {
                    Ok(resp) if resp.status == "ok" => BridgeMessage::ChartExported(resp),
                    Ok(resp) => BridgeMessage::Error(resp.message),
                    Err(e) => BridgeMessage::Error(format!("Erro ao exportar gráfico: {e}")),
                },
                Err(err) => BridgeMessage::Error(err),
            };
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }
}


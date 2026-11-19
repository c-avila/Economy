"""
Local Cache & Persistent Storage Module (SQLite) — DAX v0.3.

Data classes and refresh policy:
- DAILY  (stocks, FX, world indices): short TTL, refreshed every 10 min by the GUI.
- MONTHLY (policy rates, CPI, unemployment, PMI, ...): valid for the calendar month.
- STRUCTURAL (GDP, population): stored permanently, refreshed monthly.

Integrity rules:
1. Synthetic/fallback data NEVER overwrites a previously stored real payload.
2. When the network fails, the last known REAL payload (even if expired) is preferred
   over synthetic data.
3. WAL journal mode + retries allow several Python processes to write concurrently.
"""

import json
import os
import sqlite3
import time
from pathlib import Path
from typing import Any, Optional

BASE_DIR = Path(__file__).resolve().parent.parent
_env_data_dir = os.environ.get("DAX_DATA_DIR", "").strip()
DATA_DIR = Path(_env_data_dir) if _env_data_dir else (BASE_DIR / "data")
CACHE_DB_PATH = DATA_DIR / "cache.db"

_DB_READY = False


def _conectar() -> sqlite3.Connection:
    conn = sqlite3.connect(CACHE_DB_PATH, timeout=20.0)
    try:
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA synchronous=NORMAL")
        conn.execute("PRAGMA busy_timeout=20000")
    except Exception:
        pass
    return conn


def inicializar_banco_cache() -> None:
    """Creates the data directory and cache table if they do not exist (with schema migration)."""
    global _DB_READY
    if _DB_READY:
        return
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    with _conectar() as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS api_cache (
                cache_key TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                updated_at REAL NOT NULL
            )
            """
        )
        colunas = {row[1] for row in conn.execute("PRAGMA table_info(api_cache)").fetchall()}
        if "is_real" not in colunas:
            conn.execute("ALTER TABLE api_cache ADD COLUMN is_real INTEGER NOT NULL DEFAULT 1")
        conn.commit()
    _DB_READY = True


def obter_cache(
    cache_key: str,
    ttl_seconds: int = 1800,
    permitir_expirado: bool = False,
    somente_real: bool = False,
) -> Optional[dict[str, Any]]:
    """
    Reads a cached payload.
    - ttl_seconds: freshness window.
    - permitir_expirado: return even if older than TTL (last-known-good fallback).
    - somente_real: ignore entries that were produced from synthetic/fallback data.
    """
    try:
        inicializar_banco_cache()
        with _conectar() as conn:
            row = conn.execute(
                "SELECT payload_json, updated_at, is_real FROM api_cache WHERE cache_key = ?",
                (cache_key,),
            ).fetchone()
        if not row:
            return None
        payload_str, updated_at, is_real = row
        if somente_real and not int(is_real or 0):
            return None
        idade = time.time() - float(updated_at)
        if idade <= ttl_seconds or permitir_expirado:
            dados = json.loads(payload_str)
            if isinstance(dados, dict):
                dados["_cache_hit"] = True
                dados["_cache_age_sec"] = round(idade, 1)
                dados["_cache_is_real"] = bool(int(is_real or 0))
                dados["_cache_saved_at"] = time.strftime("%d/%m/%Y %H:%M:%S", time.localtime(float(updated_at)))
            return dados
    except Exception:
        return None
    return None


def salvar_cache(
    cache_key: str,
    category: str,
    payload: dict[str, Any],
    is_real: bool = True,
) -> bool:
    """
    Saves a payload. A synthetic/fallback payload (is_real=False) never overwrites
    an existing REAL payload for the same key. Retries on 'database is locked'.
    Returns True if written.
    """
    payload_limpo = {k: v for k, v in payload.items() if not str(k).startswith("_cache_")}
    json_str = json.dumps(payload_limpo, ensure_ascii=False)

    for tentativa in range(5):
        try:
            inicializar_banco_cache()
            with _conectar() as conn:
                if not is_real:
                    existente = conn.execute(
                        "SELECT is_real FROM api_cache WHERE cache_key = ?", (cache_key,)
                    ).fetchone()
                    if existente and int(existente[0] or 0):
                        return False
                conn.execute(
                    """
                    INSERT INTO api_cache (cache_key, category, payload_json, updated_at, is_real)
                    VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT(cache_key) DO UPDATE SET
                        category = excluded.category,
                        payload_json = excluded.payload_json,
                        updated_at = excluded.updated_at,
                        is_real = excluded.is_real
                    """,
                    (cache_key, category, json_str, time.time(), 1 if is_real else 0),
                )
                conn.commit()
            return True
        except sqlite3.OperationalError:
            time.sleep(0.25 * (tentativa + 1))
        except Exception:
            return False
    return False


def limpar_cache(categoria: str | None = None, preservar_longo_prazo: bool = True) -> int:
    """
    Clears cached entries and returns the count removed.
    By default (`preservar_longo_prazo=True`), preserves permanent long-term macroeconomic
    and policy interest rate series while clearing daily stock/FX/overview cache.
    """
    try:
        inicializar_banco_cache()
        with _conectar() as conn:
            if categoria:
                cursor = conn.execute("DELETE FROM api_cache WHERE category = ?", (categoria,))
            elif preservar_longo_prazo:
                cursor = conn.execute(
                    "DELETE FROM api_cache WHERE category NOT IN ('macro_wb', 'macro_permanent', 'interest_rates', 'interest_permanent')"
                )
            else:
                cursor = conn.execute("DELETE FROM api_cache")
            conn.commit()
            return int(cursor.rowcount or 0)
    except Exception:
        return 0


def resumo_armazenamento() -> list[dict[str, Any]]:
    """Returns per-category storage status (count, newest update, real vs fallback entries)."""
    try:
        inicializar_banco_cache()
        with _conectar() as conn:
            rows = conn.execute(
                """
                SELECT category, COUNT(*), MAX(updated_at), SUM(is_real)
                FROM api_cache GROUP BY category ORDER BY category
                """
            ).fetchall()
        return [
            {
                "category": cat,
                "entries": int(n),
                "real_entries": int(reais or 0),
                "last_update": time.strftime("%d/%m/%Y %H:%M:%S", time.localtime(float(ult))) if ult else "",
            }
            for cat, n, ult, reais in rows
        ]
    except Exception:
        return []


def is_offline_mode() -> bool:
    """Returns True if DAX_OFFLINE_MODE=1 is set in the environment."""
    return os.environ.get("DAX_OFFLINE_MODE", "").strip() in ("1", "true", "TRUE", "yes")


def end_date_inclusivo(end_date: str) -> str:
    """yfinance treats `end` as exclusive; returns end_date + 1 day so today's session is included."""
    from datetime import datetime, timedelta

    try:
        return (datetime.strptime(end_date.strip(), "%Y-%m-%d") + timedelta(days=1)).strftime("%Y-%m-%d")
    except Exception:
        return end_date

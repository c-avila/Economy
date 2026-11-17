"""
Local Cache Management Module (SQLite).
Stores JSON responses from financial and macroeconomic APIs to avoid
repeated requests, speed up navigation, and enable resilient offline fallback.
"""

import json
import os
import sqlite3
import time
from pathlib import Path
from typing import Any, Optional

# Default path for the SQLite cache database
BASE_DIR = Path(__file__).resolve().parent.parent
_env_data_dir = os.environ.get("DAX_DATA_DIR", "").strip()
DATA_DIR = Path(_env_data_dir) if _env_data_dir else (BASE_DIR / "data")
CACHE_DB_PATH = DATA_DIR / "cache.db"


def inicializar_banco_cache() -> None:
    """Cria o diretório de dados e a tabela de cache no SQLite caso não existam."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(CACHE_DB_PATH, timeout=10.0) as conn:
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
        conn.commit()


def obter_cache(
    cache_key: str,
    ttl_seconds: int = 1800,
    permitir_expirado: bool = False,
) -> Optional[dict[str, Any]]:
    """
    Busca um registro no cache SQLite.
    - ttl_seconds: tempo de vida em segundos (padrão: 30 minutos).
    - permitir_expirado: se True, retorna o dado mesmo vencido (útil como fallback de erro de rede).
    """
    try:
        inicializar_banco_cache()
        with sqlite3.connect(CACHE_DB_PATH, timeout=10.0) as conn:
            cursor = conn.execute(
                "SELECT payload_json, updated_at FROM api_cache WHERE cache_key = ?",
                (cache_key,),
            )
            row = cursor.fetchone()
            if not row:
                return None

            payload_str, updated_at = row
            idade = time.time() - float(updated_at)
            if idade <= ttl_seconds or permitir_expirado:
                dados = json.loads(payload_str)
                if isinstance(dados, dict):
                    dados["_cache_hit"] = True
                    dados["_cache_age_sec"] = round(idade, 1)
                return dados
    except Exception:
        return None
    return None


def salvar_cache(cache_key: str, category: str, payload: dict[str, Any]) -> None:
    """Salva ou atualiza um dicionário JSON no banco SQLite local."""
    try:
        inicializar_banco_cache()
        payload_limpo = dict(payload)
        payload_limpo.pop("_cache_hit", None)
        payload_limpo.pop("_cache_age_sec", None)

        json_str = json.dumps(payload_limpo, ensure_ascii=False)
        with sqlite3.connect(CACHE_DB_PATH, timeout=10.0) as conn:
            conn.execute(
                """
                INSERT INTO api_cache (cache_key, category, payload_json, updated_at)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(cache_key) DO UPDATE SET
                    category = excluded.category,
                    payload_json = excluded.payload_json,
                    updated_at = excluded.updated_at
                """,
                (cache_key, category, json_str, time.time()),
            )
            conn.commit()
    except Exception:
        pass


def limpar_cache() -> int:
    """Clears all cached entries in the local SQLite database and returns the count removed."""
    try:
        inicializar_banco_cache()
        with sqlite3.connect(CACHE_DB_PATH, timeout=10.0) as conn:
            cursor = conn.execute("DELETE FROM api_cache")
            conn.commit()
            return int(cursor.rowcount or 0)
    except Exception:
        return 0


def is_offline_mode() -> bool:
    """Returns True if DAX_OFFLINE_MODE=1 is set in the environment."""
    return os.environ.get("DAX_OFFLINE_MODE", "").strip() in ("1", "true", "TRUE", "yes")



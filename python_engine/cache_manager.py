"""
Módulo de Gerenciamento de Cache Local (SQLite).
Armazena respostas JSON das APIs financeiras e macroeconômicas para evitar
requisições repetidas, acelerar a navegação e permitir funcionamento resiliente.
"""

import json
import sqlite3
import time
from pathlib import Path
from typing import Any, Optional

# Caminho padrão para o banco SQLite de cache
BASE_DIR = Path(__file__).resolve().parent.parent
DATA_DIR = BASE_DIR / "data"
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


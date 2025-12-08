#!/usr/bin/env python3
import argparse
import json
import random
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Dict, Iterable, List

DEFAULT_MAX_EXAMPLES_PER_KEY = 5
DEFAULT_MAX_STRING_EXAMPLE_LEN = 200
DEFAULT_MAX_DEPTH = 3
DEFAULT_MAX_ARRAY_ITEMS = 5
MAX_OBJECT_KEYS_IN_SUMMARY = 20


# ---------- 入力読み取り (JSON / JSONL 共通) ----------

def iter_records_from_file(path: Path) -> Iterable[Dict[str, Any]]:
    """
    1. まず jsonl (1行1JSON) として試す。
    2. だめならファイル全体を JSON として読む。
       - 配列なら各要素(dictのみ)をレコードとして扱う。
       - オブジェクトならそのまま1レコード。
    """
    text = path.read_text(encoding="utf-8", errors="ignore")

    # jsonl 試行
    records: List[Dict[str, Any]] = []
    jsonl_ok = True
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
            if isinstance(obj, dict):
                records.append(obj)
            else:
                jsonl_ok = False
                records = []
                break
        except json.JSONDecodeError:
            jsonl_ok = False
            records = []
            break

    if jsonl_ok and records:
        for r in records:
            yield r
        return

    # 通常 JSON
    try:
        obj = json.loads(text)
    except json.JSONDecodeError:
        sys.stderr.write(f"[WARN] Failed to parse {path} as JSON/JSONL\n")
        return

    if isinstance(obj, list):
        for item in obj:
            if isinstance(item, dict):
                yield item
    elif isinstance(obj, dict):
        yield obj
    else:
        sys.stderr.write(f"[WARN] Unsupported JSON root type in {path}: {type(obj)}\n")


# ---------- 型・サマリ関連 ----------

def type_name(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "bool"
    if isinstance(value, (int, float)):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    if isinstance(value, dict):
        return "object"
    return type(value).__name__


def summarize_value(v: Any, max_string_len: int) -> Any:
    """
    例として保持する値を軽量化するためのサマリ関数。
    - string: 先頭 max_string_len 文字まで、残りは truncated 表示
    - array: 実体は持たず、長さと先頭要素の型だけ
    - object: 実体は持たず、キー一覧だけ（最大 MAX_OBJECT_KEYS_IN_SUMMARY 個）
    - それ以外: そのまま（number / bool / null など）
    """
    if isinstance(v, str):
        if len(v) <= max_string_len:
            return v
        return v[:max_string_len] + f"...(truncated, original_len={len(v)})"

    if isinstance(v, list):
        sample_type = None
        if v:
            sample_type = type_name(v[0])
        return {
            "__kind__": "array",
            "len": len(v),
            "sample_type": sample_type,
        }

    if isinstance(v, dict):
        keys = list(v.keys())[:MAX_OBJECT_KEYS_IN_SUMMARY]
        return {
            "__kind__": "object",
            "keys": keys,
        }

    # number / bool / null などはそのまま
    return v


# ---------- プロファイル本体 ----------

def ensure_field_stats(
    stats: Dict[str, Dict[str, Any]],
    path: str,
) -> Dict[str, Any]:
    return stats.setdefault(
        path,
        {
            "count": 0,
            "types": Counter(),
            "examples": [],
            "string_length_stats": {
                "min": None,
                "max": None,
                "sum": 0,
                "count": 0,
            },
        },
    )


def update_field_stats(
    stats: Dict[str, Dict[str, Any]],
    path: str,
    value: Any,
    max_examples_per_key: int,
    max_string_example_len: int,
) -> None:
    st = ensure_field_stats(stats, path)
    st["count"] += 1
    st["types"][type_name(value)] += 1

    # 文字列長の統計
    if isinstance(value, str):
        l = len(value)
        sls = st["string_length_stats"]
        if sls["min"] is None or l < sls["min"]:
            sls["min"] = l
        if sls["max"] is None or l > sls["max"]:
            sls["max"] = l
        sls["sum"] += l
        sls["count"] += 1

    # 例（軽量版）
    if len(st["examples"]) < max_examples_per_key:
        st["examples"].append(summarize_value(value, max_string_example_len))


def walk_value(
    stats: Dict[str, Dict[str, Any]],
    path: str,
    value: Any,
    depth: int,
    max_depth: int,
    max_examples_per_key: int,
    max_string_example_len: int,
    max_array_items: int,
) -> None:
    """
    value に対して:
      - 現在の path を1フィールドとしてプロファイル
      - 可能ならネストにも降りていき、path を "a.b", "messages[].content" のようにして集計
    """
    if depth > max_depth:
        return

    # 現在のフィールドを更新
    update_field_stats(
        stats,
        path,
        value,
        max_examples_per_key,
        max_string_example_len,
    )

    # ネストを追うのは object / array のみ
    if isinstance(value, dict):
        if depth == max_depth:
            return
        for k, v in value.items():
            child_path = f"{path}.{k}" if path else k
            walk_value(
                stats,
                child_path,
                v,
                depth + 1,
                max_depth,
                max_examples_per_key,
                max_string_example_len,
                max_array_items,
            )
    elif isinstance(value, list):
        if depth == max_depth:
            return
        # 配列の要素には "[]" をつけて表現
        child_path = f"{path}[]" if path else "[]"
        for i, elem in enumerate(value):
            if i >= max_array_items:
                break
            walk_value(
                stats,
                child_path,
                elem,
                depth + 1,
                max_depth,
                max_examples_per_key,
                max_string_example_len,
                max_array_items,
            )


def profile_root(
    root: Path,
    max_examples_per_key: int,
    max_string_example_len: int,
    sample_rate: float,
    max_depth: int,
    max_array_items: int,
) -> Dict[str, Any]:
    field_stats: Dict[str, Dict[str, Any]] = {}
    total_records = 0
    total_records_seen = 0  # サンプリング前のカウンタ

    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix not in (".jsonl", ".json"):
            continue

        for record in iter_records_from_file(path):
            if not isinstance(record, dict):
                continue

            total_records_seen += 1
            if sample_rate < 1.0:
                if random.random() > sample_rate:
                    continue

            total_records += 1

            # トップレベルの各キーからスタート
            for k, v in record.items():
                walk_value(
                    stats=field_stats,
                    path=k,
                    value=v,
                    depth=0,
                    max_depth=max_depth,
                    max_examples_per_key=max_examples_per_key,
                    max_string_example_len=max_string_example_len,
                    max_array_items=max_array_items,
                )

    # coverage / 平均文字列長を計算
    result_keys: Dict[str, Any] = {}
    for path, st in field_stats.items():
        count = st["count"]
        types_counter: Counter = st["types"]
        sls = st["string_length_stats"]
        avg_len = None
        if sls["count"] > 0:
            avg_len = sls["sum"] / sls["count"]

        result_keys[path] = {
            "count": count,
            "coverage": count / total_records if total_records > 0 else 0.0,
            "types": dict(types_counter),
            "examples": st["examples"],
            "string_length_stats": {
                "min": sls["min"],
                "max": sls["max"],
                "avg": avg_len,
                "count": sls["count"],
            },
        }

    return {
        "total_records": total_records,
        "total_records_seen": total_records_seen,
        "sample_rate": sample_rate,
        "max_depth": max_depth,
        "max_array_items": max_array_items,
        "keys": result_keys,
    }


# ---------- CLI ----------

def main():
    parser = argparse.ArgumentParser(
        description=(
            "Profile JSON/JSONL agent logs (nested keys, types, coverage, examples, "
            "string length stats). Paths like 'message.role', 'payload.type', "
            "'messages[].content' を集計します。"
        )
    )
    parser.add_argument("--root", type=str, required=True, help="Root directory")
    parser.add_argument("--label", type=str, default=None, help="Optional dataset label")
    parser.add_argument("--out", type=str, default=None, help="Output file (default: stdout)")
    parser.add_argument(
        "--max-examples-per-key",
        type=int,
        default=DEFAULT_MAX_EXAMPLES_PER_KEY,
        help="Max number of example values per key path",
    )
    parser.add_argument(
        "--max-string-example-len",
        type=int,
        default=DEFAULT_MAX_STRING_EXAMPLE_LEN,
        help="Max characters to keep for string examples (will be truncated)",
    )
    parser.add_argument(
        "--sample-rate",
        type=float,
        default=1.0,
        help="Record sampling rate in [0,1]. e.g. 0.1 = 10%% of records",
    )
    parser.add_argument(
        "--max-depth",
        type=int,
        default=DEFAULT_MAX_DEPTH,
        help="Max nesting depth to profile (0 = only top-level)",
    )
    parser.add_argument(
        "--max-array-items",
        type=int,
        default=DEFAULT_MAX_ARRAY_ITEMS,
        help="Max number of array elements to traverse per array value",
    )

    args = parser.parse_args()

    root = Path(args.root).expanduser()
    if not root.exists():
        sys.stderr.write(f"[ERROR] root not found: {root}\n")
        sys.exit(1)

    profile = profile_root(
        root=root,
        max_examples_per_key=args.max_examples_per_key,
        max_string_example_len=args.max_string_example_len,
        sample_rate=args.sample_rate,
        max_depth=args.max_depth,
        max_array_items=args.max_array_items,
    )
    if args.label:
        profile["label"] = args.label

    out_text = json.dumps(profile, ensure_ascii=False, indent=2)
    if args.out:
        Path(args.out).write_text(out_text, encoding="utf-8")
    else:
        print(out_text)


if __name__ == "__main__":
    main()

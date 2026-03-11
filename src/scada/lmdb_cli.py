import argparse
import os
from pathlib import Path

import lmdb


def is_lmdb_env(path: Path) -> bool:
    """Check if a directory contains LMDB data files."""
    if path.is_dir():
        # Most LMDB environments have a data.mdb file
        return (path / "data.mdb").exists()
    return False


def inspect_lmdb(env_path: Path, show_keys: bool = False, limit: int = 10):
    try:
        env = lmdb.open(
            str(env_path), readonly=True, create=False, max_dbs=128, lock=False
        )
        print(f"=== LMDB Environment: {env_path} ===")

        # Get environment stats
        stat = env.stat()
        info = env.info()

        print("Environment Info:")
        for k, v in info.items():
            print(f"  {k}: {v}")

        print("Environment Stats:")
        for k, v in stat.items():
            print(f"  {k}: {v}")

        with env.begin() as txn:
            named_dbs = {}
            main_entries = []

            # Find named databases vs normal keys in the main DB
            with txn.cursor() as cursor:
                for key, value in cursor:
                    try:
                        db = env.open_db(key, txn=txn, create=False)
                        named_dbs[key] = db
                    except Exception:
                        main_entries.append((key, value))

            if named_dbs:
                print("\nSub-Databases Found:")
                for db_name, db in named_dbs.items():
                    name_str = db_name.decode("utf-8", errors="replace")
                    print(f"  - {name_str}")
                    try:
                        db_stat = txn.stat(db)
                        for k, v in db_stat.items():
                            print(f"      {k}: {v}")
                    except Exception as e:
                        print(f"      (Could not get stats: {e})")

            if show_keys:
                print("\n--- Entries ---")
                if main_entries or not named_dbs:
                    print("Main Environment Entries (Default DB):")
                    count = 0
                    for key, value in main_entries:
                        if limit > 0 and count >= limit:
                            print(f"  ... (limited to {limit} entries)")
                            break

                        key_str = repr(key)
                        val_str = repr(value)
                        if len(val_str) > 100:
                            val_str = val_str[:97] + "..."

                        print(f"  {key_str}: {val_str}")
                        count += 1
                    if count == 0:
                        print("  (Empty)")

                for db_name, db in named_dbs.items():
                    name_str = db_name.decode("utf-8", errors="replace")
                    print(f"\nSub-Database: {name_str}")
                    with txn.cursor(db) as cursor:
                        count = 0
                        for key, value in cursor:
                            if limit > 0 and count >= limit:
                                print(f"  ... (limited to {limit} entries)")
                                break

                            key_str = repr(key)
                            val_str = repr(value)
                            if len(val_str) > 100:
                                val_str = val_str[:97] + "..."

                            print(f"  {key_str}: {val_str}")
                            count += 1
                        if count == 0:
                            print("  (Empty)")

        print()
        env.close()
    except Exception as e:
        print(f"Error reading LMDB at {env_path}: {e}\n")


def main():
    parser = argparse.ArgumentParser(
        description="Inspect LMDB databases in a directory."
    )
    parser.add_argument(
        "path",
        type=str,
        help="Directory to scan for LMDB databases, "
        "or path to a specific LMDB directory.",
    )
    parser.add_argument(
        "--keys", action="store_true", help="Show a sample of keys/values."
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=10,
        help="Maximum number of keys to display per database "
        "(default: 10. Use 0 for unlimited).",
    )
    args = parser.parse_args()

    target_path = Path(args.path)

    if not target_path.exists():
        print(f"Error: Path '{target_path}' does not exist.")
        return

    # If the user points directly to an LMDB environment directory
    if is_lmdb_env(target_path):
        inspect_lmdb(target_path, args.keys, args.limit)
    elif target_path.is_dir():
        # Otherwise, scan the directory for LMDB environments
        found = False
        for root, dirs, files in os.walk(target_path):
            current_dir = Path(root)
            if is_lmdb_env(current_dir):
                found = True
                inspect_lmdb(current_dir, args.keys, args.limit)

        if not found:
            print(f"No LMDB databases found in '{target_path}'.")
    else:
        print(f"Path '{target_path}' is neither a directory nor an LMDB environment.")


if __name__ == "__main__":
    main()

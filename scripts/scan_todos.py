#!/usr/bin/env python3
"""
Scan project files for TODO comments and generate TODO.json

TODO comment format: // TODO: (FEATURE_ID) or /// TODO: (FEATURE_ID)
Feature IDs are validated against FEATURE_LIST.yaml
"""

import json
import os
import re
from pathlib import Path
from typing import List, Dict, Set


def load_feature_ids(feature_list_path: str) -> Set[str]:
    """Parse FEATURE_LIST.yaml and return a set of valid feature IDs"""
    feature_ids = set()
    pattern = re.compile(r'^\s*-\s*id:\s+([A-Z0-9-]+)')

    with open(feature_list_path, 'r', encoding='utf-8') as f:
        for line in f:
            match = pattern.match(line)
            if match:
                feature_ids.add(match.group(1))

    return feature_ids


def scan_file_for_todos(file_path: str, valid_features: Set[str]) -> List[Dict]:
    """Scan a single file for TODO comments"""
    todos = []

    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except Exception as e:
        print(f"Warning: Could not read {file_path}: {e}")
        return todos

    # Pattern for TODO comments: // TODO: (FEATURE_ID) or /// TODO: (FEATURE_ID)
    # Also supports //! and ///  (doc comments)
    pattern = re.compile(r'^\s*(///?|//)\s*TODO:\s*\(([^)]+)\)\s*(.*)$')

    i = 0
    while i < len(lines):
        line = lines[i]
        match = pattern.match(line)

        if match:
            comment_prefix = match.group(1)
            feature_ids_str = match.group(2)
            description = match.group(3).strip()

            # Parse feature IDs (comma-separated)
            feature_ids = [f.strip() for f in feature_ids_str.split(',')]

            # Validate feature IDs
            invalid_ids = [f for f in feature_ids if f not in valid_features]
            if invalid_ids:
                print(f"Warning: {file_path}:{i+1}: Invalid feature IDs: {invalid_ids}")

            # Find the full description (might span multiple lines)
            full_description = description
            j = i + 1
            while j < len(lines):
                next_line = lines[j].strip()
                # Check if the next line is a continuation comment
                if next_line.startswith('///') or next_line.startswith('//'):
                    continuation = re.sub(r'^///?\s*', '', next_line).strip()
                    if continuation and not continuation.startswith('TODO:'):
                        full_description += ' ' + continuation
                        j += 1
                    else:
                        break
                else:
                    break

            todos.append({
                'file': os.path.relpath(file_path, os.getcwd()),
                'line': f"{i + 1}-{j}",
                'description': full_description or "No description provided",
                'dependencies': feature_ids
            })
            i = j
        else:
            i += 1

    return todos


def scan_project(root_dir: str, extensions: List[str], valid_features: Set[str]) -> List[Dict]:
    """Scan all files in the project for TODO comments"""
    all_todos = []

    for ext in extensions:
        for file_path in Path(root_dir).rglob(f'*{ext}'):
            # Skip target directory and build artifacts
            if 'target' in str(file_path) or '.git' in str(file_path):
                continue

            todos = scan_file_for_todos(str(file_path), valid_features)
            all_todos.extend(todos)

    return all_todos


def main():
    # Determine project root
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)

    # Paths
    feature_list_path = os.path.join(project_root, 'FEATURE_LIST.yaml')
    output_path = os.path.join(project_root, 'TODO.json')

    # Load feature list
    print(f"Loading feature list from {feature_list_path}")
    valid_features = load_feature_ids(feature_list_path)
    print(f"Found {len(valid_features)} valid feature IDs")

    # Scan for TODOs
    print(f"Scanning project for TODO comments...")
    extensions = ['.rs']  # Rust files
    todos = scan_project(project_root, extensions, valid_features)

    # Sort by file and line number
    todos.sort(key=lambda x: (x['file'], int(x['line'].split('-')[0])))

    # Write output
    print(f"Found {len(todos)} TODO comments")
    print(f"Writing to {output_path}")

    output_data = []
    for todo in todos:
        output_data.append({
            'file': todo['file'],
            'line': todo['line'],
            'description': todo['description'],
            'dependencies': todo['dependencies']
        })

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(output_data, f, indent=2, ensure_ascii=False)

    print("Done!")


if __name__ == '__main__':
    main()

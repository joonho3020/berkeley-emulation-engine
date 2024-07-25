#!/usr/bin/env python3

import argparse
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description='Runner for blif parser')
parser.add_argument('--blif', type=Path, help='path to input blif file')
parser.add_argument('--dot', action='store_true', help='output related dotfiles into human viewable format')
parser.add_argument('--test', action='store_true', help='run cargo tests')
args = parser.parse_args()

def run(cmd: str) -> subprocess.CompletedProcess:
  return subprocess.run(cmd, shell = True)

def run_compiler():
  run(f'cargo run -- {args.blif}')

def run_test():
  run(f'cargo test')

def generate_human_readable_graphs():
  blif_path: Path = args.blif
  dotfiles = blif_path.parent.glob(f'*{blif_path.name}-*.dot')
  for df in list(dotfiles):
    run(f'dot {df} -Tpdf -Kdot > {blif_path.parent}/{df.name}.pdf')
# run(f'tar -cvzf {blif_path.parent}/{blif_path.name}.tar.gz {blif_path.parent}/{blif_path.name}*')
# run(f'rm {blif_path.parent}/{blif_path.name}-*')

def main():
  if args.test:
    run_test()
    return
  else:
    run_compiler()
    if args.dot:
      generate_human_readable_graphs()

if __name__=="__main__":
  main()

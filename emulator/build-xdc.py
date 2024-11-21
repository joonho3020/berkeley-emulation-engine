from argparse import ArgumentParser
from typing import Dict, List, Optional, Tuple
import json
from enum import Enum

parser = ArgumentParser(description='Generate XDC files for efficient FPGA implementation')
parser.add_argument('--num-mods',          type=int, required=True)
parser.add_argument('--num-procs',         type=int, required=True)
parser.add_argument('--max-steps',         type=int, required=True)
parser.add_argument('--inter-mod-nw-lat',  type=int, required=True)
parser.add_argument('--inter-proc-nw-lat', type=int, required=True)
parser.add_argument('--sram-width',        type=int, required=True)
parser.add_argument('--sram-entries',      type=int, required=True)
args = parser.parse_args()


def generated_dir(args) -> str:
  return f'generated-m{args.num_mods}.p{args.num_procs}.s{args.max_steps}.nwl{args.inter_mod_nw_lat}.nwg{args.inter_proc_nw_lat}.sw{args.sram_width}.se{args.sram_entries}'


def read_firtool_input_annos() -> List:
  with open('annos.json', 'r') as file:
    data = json.load(file)
    return data

def top_module_hierarchy_file(firtool_annos: List) -> Optional[str]:
  for anno in firtool_annos:
    if anno['class'] == "sifive.enterprise.firrtl.ModuleHierarchyAnnotation":
      return anno['filename']
  return None

def read_module_hierarchy(args) -> Dict:
  firtool_annos = read_firtool_input_annos()
  tmhf = top_module_hierarchy_file(firtool_annos)

  if tmhf == None:
    print(f'No firtool input annotation for top hierarchy file found')
    exit(1)

  hier: Dict = dict()
  with open(f'{generated_dir(args)}/{tmhf}', 'r') as file:
    hier = json.load(file)
  return hier


ModInst = Tuple[str, str]
HierarchyPath = List[ModInst]

# Perform DFS to find all the instances of a module
# and return all the possible HierarchyPath to the instance
def get_hierarchy_path(hier: Dict, module: str) -> List[HierarchyPath]:
  cur_mod_inst: ModInst = (hier['module_name'], hier['instance_name'])
  if hier['module_name'] == module:
    return [[cur_mod_inst]]

  instances = hier['instances']
  ret: List[HierarchyPath] = list()
  for child_instance in instances:
    child_path = get_hierarchy_path(child_instance, module)
    if len(child_path) == 0:
      continue
    else:
      for cp in child_path:
        cp_ = cp.copy()
        cp_.append(cur_mod_inst)
        ret.append(cp_)
  return ret

# Yeah I know, I can use StrEnum, but I have to upgrade to python 3.11 which
# I'm too lazy for atm
class RAMStyle(Enum):
    BLOCK = 1
    ULTRA = 2

def ramstyle_str(st: RAMStyle) -> str:
  if st == RAMStyle.BLOCK:
    return "BLOCK"
  else:
    return "ULTRA"


def memory_constraints(hpath: HierarchyPath, style: RAMStyle) -> str:
  inst_path = ''
  for mi in reversed(hpath):
    inst_path += mi[1] + '/'
  xdc = f'set_property RAM_STYLE {ramstyle_str(style)} [get_cells -hierarchical -regexp .*{inst_path}.*]'
  return xdc

def main():
  hier = read_module_hierarchy(args)

  module_to_find = f'sram_{args.sram_entries}x{args.sram_width}'

  paths = get_hierarchy_path(hier, module_to_find) 
  with open(f'{generated_dir(args)}/synth.xdc', 'w') as f:
    for p in paths:
      xdc = memory_constraints(p, RAMStyle.ULTRA)
      f.write(f'{xdc}\n')

if __name__=="__main__":
  main()

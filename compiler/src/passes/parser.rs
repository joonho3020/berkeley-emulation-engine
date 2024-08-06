use crate::primitives::*;
use indexmap::IndexMap;
use petgraph::graph::NodeIndex;
use std::fs;

type IResultStr<'a> = IResult<&'a str, &'a str>;

use nom::{
    bytes::complete::{is_not, tag, take_until},
    combinator::value,
    sequence::{pair, terminated},
    IResult,
};

fn take_until_or_end<'a>(tag: &'a str, istr: &'a str) -> IResultStr<'a> {
    let ret: IResult<&str, &str> = take_until(tag)(istr);
    match ret {
        Ok(x) => Ok(x),
        Err(_) => Ok(("", istr)),
    }
}

fn terminated_newline<'a>(istr: &'a str) -> IResultStr<'a> {
    let ret: IResult<&str, &str> =
        terminated(take_until("\n"), nom::character::complete::newline)(istr);
    match ret {
        Ok(x) => Ok(x),
        Err(_) => Ok(("", istr)),
    }
}

fn lut_table_parser<'a>(input: &'a str, table: &mut Vec<Vec<u8>>) -> IResultStr<'a> {
    let mut i = input;
    let mut li;
    let mut te;
    while i.len() > 0 {
        (i, li) = terminated_newline(i)?;

        let mut row: Vec<u8> = vec![];
        let (_, mut table_input) = take_until(" ")(li)?;
        while table_input.len() > 0 {
            (table_input, te) = nom::character::complete::one_of("01")(table_input)?;
            row.push(te.to_digit(10).unwrap() as u8);
        }
        table.push(row);
    }
    Ok(("", ""))
}

fn lut_body_parser<'a>(input: &'a str, luts: &mut Vec<Lut>) -> IResultStr<'a> {
    let (i, ioline) = terminated_newline(input)?;
    let mut io: Vec<&str> = ioline.split(' ').collect();

    let output = io.pop().unwrap_or("INVALID_OUTPUT").to_string();
    let inputs: Vec<String> = io.iter().map(|v| v.to_string()).collect();
    let (i, table) = take_until_or_end(".", i)?;

    let mut lut_table = vec![];
    let _ = lut_table_parser(table, &mut lut_table);

    luts.push(Lut {
        inputs: inputs,
        output: output,
        table: lut_table,
        info: NodeInfo::default(),
    });

    Ok((i, ""))
}

fn subckt_parser<'a>(input: &'a str, subckts: &mut Vec<Subckt>) -> IResultStr<'a> {
    let (i, sline) = terminated_newline(input)?;
    let conns_vec: Vec<&str> = sline.split(' ').collect();
    let name = conns_vec[0];

    let mut conns = IndexMap::new();
    conns_vec.iter().skip(1).for_each(|c| {
        let lr: Vec<&str> = c.split('=').collect();
        let lhs = lr[0];
        let rhs = lr[1];
        conns.insert(lhs.to_string(), rhs.to_string());
    });

    subckts.push(Subckt {
        name: name.to_string(),
        conns: conns,
        info: NodeInfo::default(),
    });

    Ok((i, ""))
}

// _SDFF_NP0_ : FF with reset C D Q R
// _DFFE_PN_  : FF with enables C D E Q
// _SDFFE_PP0N_ : FF with reset and enable C D E Q R
fn gate_parser<'a>(input: &'a str, gates: &mut Vec<Gate>) -> IResultStr<'a> {
    let (i, line) = terminated_newline(input)?;
    let signal_conns: Vec<&str> = line.split(' ').collect();
    let mut gate = Gate::default();

    for sc in signal_conns.iter() {
        let x: Vec<&str> = sc.split('=').collect();
        if x.len() != 2 {
            continue;
        }
        match x[0] {
            "C" => {
                gate.c = x[1].to_string();
            }
            "D" => {
                gate.d = x[1].to_string();
            }
            "Q" => {
                gate.q = x[1].to_string();
            }
            "R" => {
                gate.r = Some(x[1].to_string());
            }
            "E" => {
                gate.e = Some(x[1].to_string());
            }
            _ => {}
        }
    }
    gates.push(gate);
    Ok((i, ""))
}

fn latch_parser<'a>(input: &'a str, latches: &mut Vec<Latch>) -> IResultStr<'a> {
    let (i, line) = terminated_newline(input)?;
    let latch_info: Vec<&str> = line.split(' ').collect();
    let mut latch = Latch::default();

    for (idx, li) in latch_info.iter().enumerate() {
        match idx {
            0 => {
                latch.input = li.to_string();
            }
            1 => {
                latch.output = li.to_string();
            }
            3 => {
                latch.control = li.to_string();
            }
            4 => {
                latch.init = LatchInit::to_enum(li);
            }
            _ => {}
        }
    }
    latches.push(latch);
    Ok((i, ""))
}

fn module_body_parser<'a>(input: &'a str, circuit: &mut Circuit) -> IResultStr<'a> {
    let body_end_marker = "\n.end\n";

    let mut net_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();
    let mut out_to_nodeidx: IndexMap<String, NodeIndex> = IndexMap::new();

    // Get module body
    let (i, _) = tag(".model ")(input)?;
    let (i, _name) = terminated(take_until("\n"), nom::character::complete::newline)(i)?;
    let (mut i, body) = terminated(
        take_until(body_end_marker),
        nom::character::complete::newline,
    )(i)?;

    // Parse inputs
    let (bi, iline) = terminated(take_until("\n"), nom::character::complete::newline)(body)?;
    let inputs: Vec<String> = iline.split(' ').map(|v| v.to_string()).skip(1).collect();
    for i in inputs.iter() {
        let nidx = circuit.graph.add_node(Box::new(Input {
            name: i.clone(),
            info: NodeInfo::default(),
        }));
        circuit.io_i.insert(nidx, i.to_string());
        net_to_nodeidx.insert(i.to_string(), nidx);
    }

    // Parse outputs
    let (bi, oline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let outputs: Vec<String> = oline.split(' ').map(|v| v.to_string()).skip(1).collect();
    for o in outputs.iter() {
        let nidx = circuit.graph.add_node(Box::new(Output {
            name: o.clone(),
            info: NodeInfo::default(),
        }));
        circuit.io_o.insert(nidx, o.to_string());
        out_to_nodeidx.insert(o.to_string(), nidx);
    }

    let mut luts = vec![];
    let mut subckts = vec![];
    let mut gates = vec![];
    let mut latches = vec![];
    let mut bi = bi;
    let mut tagstr;

    while bi.len() > 1 {
        (bi, tagstr) = terminated(take_until(" "), nom::character::complete::multispace0)(bi)?;
        if tagstr.eq(".names") {
            (bi, _) = lut_body_parser(bi, &mut luts)?;
        } else if tagstr.eq(".subckt") {
            (bi, _) = subckt_parser(bi, &mut subckts)?;
        } else if tagstr.eq(".gate") {
            (bi, _) = gate_parser(bi, &mut gates)?;
        } else if tagstr.eq(".latch") {
            (bi, _) = latch_parser(bi, &mut latches)?;
        }
    }

    for lut in luts.iter() {
        let nidx = circuit.graph.add_node(Box::new(lut.clone()));
        net_to_nodeidx.insert(lut.output.to_string(), nidx);
    }

    for gate in gates.iter() {
        let nidx = circuit.graph.add_node(Box::new(gate.clone()));
        net_to_nodeidx.insert(gate.q.to_string(), nidx);
    }

    for latch in latches.iter() {
        let nidx = circuit.graph.add_node(Box::new(latch.clone()));
        net_to_nodeidx.insert(latch.output.to_string(), nidx);
    }

    for lut in luts.iter() {
        for inet in lut.inputs.iter() {
            let src_nidx = net_to_nodeidx.get(inet).unwrap();
            let dst_nidx = net_to_nodeidx.get(&lut.output).unwrap();
            circuit
                .graph
                .add_edge(*src_nidx, *dst_nidx, inet.to_string());
        }
    }

    for gate in gates.iter() {
        let d_idx = net_to_nodeidx.get(&gate.d).unwrap();
        let q_idx = net_to_nodeidx.get(&gate.q).unwrap();
        circuit.graph.add_edge(*d_idx, *q_idx, gate.d.to_string());

        match &gate.e {
            Some(e) => {
                let e_idx = net_to_nodeidx.get(e).unwrap();
                circuit.graph.add_edge(*e_idx, *q_idx, e.to_string());
            }
            None => (),
        };
    }

    for latch in latches.iter() {
        let d_idx = net_to_nodeidx.get(&latch.input).unwrap();
        let q_idx = net_to_nodeidx.get(&latch.output).unwrap();
        circuit
            .graph
            .add_edge(*d_idx, *q_idx, latch.input.to_string());
    }

    for o in outputs.iter() {
        let src_nidx = net_to_nodeidx.get(&o.to_string()).unwrap();
        let dst_nidx = out_to_nodeidx.get(&o.to_string()).unwrap();
        circuit.graph.add_edge(*src_nidx, *dst_nidx, o.to_string());
    }

    if i.len() > body_end_marker.to_string().len() {
        // Advance to the next .end
        (i, _) = take_until(".")(i)?;
    } else {
        // End of file
        (i, _) = take_until("\n")(i)?;
    }

    Ok((i, ""))
}

fn parse_modules_from_blif_str<'a>(input: &'a str, circuit: &mut Circuit) -> IResultStr<'a> {
    // remove comment
    let (i, _) = value((), pair(tag("#"), is_not("\n")))(input)?;
    let (i, _) = take_until(".")(i)?;

    let mut i = i;
    while i.len() > 4 {
        (i, _) = module_body_parser(i, circuit)?;
        (i, _) = take_until_or_end("\n.model", i)?;
        (i, _) = terminated_newline(i)?;
    }

    Ok(("", ""))
}

fn parse_blif(input: &str) -> Result<Circuit, String> {
    let mut circuit = Circuit::default();
    let res = parse_modules_from_blif_str(input, &mut circuit);
    match res {
        Ok(_) => {
            return Ok(circuit);
        }
        Err(e) => {
            return Err(format!("Error while parsing:\n{}", e).to_string());
        }
    }
}

pub fn parse_blif_file(input_file_path: &str) -> Result<Circuit, String> {
    let blif_file = fs::read_to_string(input_file_path);
    match blif_file {
        Ok(blif_str) => {
            return parse_blif(&blif_str);
        }
        Err(e) => {
            return Err(format!("Error while reading the file:\n{}", e).to_string());
        }
    }
}

#[cfg(test)]
pub mod parser_tests {
    use super::*;

    pub fn test_blif_parser(file_path: &str) -> bool {
        let res = parse_blif_file(&file_path);
        match res {
            Ok(_) => true,
            Err(err) => {
                println!("blif file parsing error:\n{}", err);
                false
            }
        }
    }

    #[test]
    pub fn test_adder() {
        assert_eq!(test_blif_parser("../examples/Adder.lut.blif"), true);
    }

    #[test]
    pub fn test_gcd() {
        assert_eq!(test_blif_parser("../examples/GCD-2bit.lut.blif"), true);
    }
}

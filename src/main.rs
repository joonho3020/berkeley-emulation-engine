

use std::{
    fmt::Debug,
    error::Error,
    env,
    fs,
    collections::HashMap
};

use nom::{
    bytes::complete::{is_not, tag, take_until, take_while},
    combinator::value,
    sequence::{pair, terminated},
    IResult
};

#[derive (Debug)]
struct Lut {
    inputs: Vec<String>,
    output: String,
    table: Vec<Vec<u8>>,
}

#[derive (Debug)]
struct Subckt {
    name: String,
    conns: HashMap<String, String>
}

#[derive (Debug)]
struct Gate {
    c: String,
    d: String,
    q: String,
    r: Option<String>,
    e: Option<String>
}

impl Default for Gate {
    fn default() -> Gate {
        Gate {
            c: "".to_string(),
            d: "".to_string(),
            q: "".to_string(),
            r: None,
            e: None
        }
    }
}

#[repr(u8)]
#[derive (Debug)]
enum LatchInit {
    ZER0 = 0,
    ONE  = 1,
    DONTCARE = 2,
    UNKNOWN  = 3,
}

impl LatchInit {
    fn to_enum(i: &str) -> LatchInit {
        match i {
            "0" => {
                LatchInit::ZER0
            }
            "1" => {
                LatchInit::ONE
            }
            "2" => {
                LatchInit::DONTCARE
            }
            _ => {
                LatchInit::UNKNOWN
            }
        }
    }
}

#[derive (Debug)]
struct Latch {
    input: String,
    output: String,
    control: String,
    init: LatchInit
}

impl Default for Latch {
    fn default() -> Latch {
        Latch {
            input: "".to_string(),
            output: "".to_string(),
            control: "".to_string(),
            init: LatchInit::UNKNOWN
        }
    }
}

pub struct Module {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
    luts: Vec<Lut>,
    subckts: Vec<Subckt>,
    gates: Vec<Gate>,
    latches: Vec<Latch>
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Module {}\n", self.name)?;

        write!(f, "  inputs: ")?;
        for i in self.inputs.iter() {
            write!(f, "  {} ", i)?;
        }
        write!(f, "\n")?;

        write!(f, "  outputs: ")?;
        for i in self.outputs.iter() {
            write!(f, "  {} ", i)?;
        }
        write!(f, "\n")?;

        for i in self.luts.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        for i in self.subckts.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        for i in self.gates.iter() {
            write!(f, "  {:?}\n", i)?;
        }
        write!(f, "\n")?;

        write!(f, "")
    }
}

fn take_until_or_end<'a>(tag: &'a str, istr: &'a str) -> IResult<&'a str, &'a str> {
    let ret: IResult<&str, &str> = take_until(tag)(istr);
    match ret {
        Ok(x) => {
            Ok(x)
        }
        Err(_) => {
            Ok(("", istr))
        }
    }
}

fn terminated_newline<'a>(istr: &'a str) -> IResult<&'a str, &'a str> {
    let ret: IResult<&str, &str> = terminated(take_until("\n"), nom::character::complete::newline)(istr);
    match ret {
        Ok(x) => {
            Ok(x)
        }
        Err(_) => {
            Ok(("", istr))
        }
    }
}

fn lut_table_parser<'a>(input: &'a str, table: &mut Vec<Vec<u8>>) -> IResult<&'a str, &'a str> {
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

fn lut_body_parser<'a>(input: &'a str, luts: &mut Vec<Lut>) -> IResult<&'a str, &'a str> {
    let (i, ioline)  = terminated_newline(input)?;
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
    });

    Ok((i, ""))
}

fn subckt_parser<'a>(input: &'a str, subckts: &mut Vec<Subckt>) -> IResult<&'a str, &'a str> {
    let (i, sline) = terminated_newline(input)?;
    let conns_vec: Vec<&str> = sline.split(' ').collect();
    let name = conns_vec[0];

    let mut conns = HashMap::new();
    conns_vec.iter().skip(1).for_each(|c| {
        let lr: Vec<&str> = c.split('=').collect();
        let lhs = lr[0];
        let rhs = lr[1];
        conns.insert(lhs.to_string(), rhs.to_string());
    });

    subckts.push(Subckt {
        name: name.to_string(),
        conns: conns
    });

    Ok((i, ""))
}

// _SDFF_NP0_ : FF with reset C D Q R
// _DFFE_PN_  : FF with enables C D E Q
// _SDFFE_PP0N_ : FF with reset and enable C D E Q R
fn gate_parser<'a>(input: &'a str, gates: &mut Vec<Gate>) -> IResult<&'a str, &'a str> {
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
            _ => {
            }
        }
    }
    gates.push(gate);
    Ok((i, ""))
}

fn latch_parser<'a>(input: &'a str, latches: &mut Vec<Latch>) -> IResult<&'a str, &'a str> {
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
                latch.init = LatchInit::to_enum(li)                                     ;
            }
            _ => {
            }
        }
    }
    latches.push(latch);
    Ok((i, ""))
}

fn module_body_parser<'a>(input: &'a str, mods: &mut Vec<Module>) -> IResult<&'a str, &'a str> {
    let body_end_marker = "\n.end\n";
    let (i, _) = tag(".model ")(input)?;
    let (i, name) = terminated(take_until("\n"),
                               nom::character::complete::newline)(i)?;
    let (mut i, body) = terminated(take_until(body_end_marker),
                            nom::character::complete::newline)(i)?;

    let (bi, iline) = terminated(take_until("\n"), nom::character::complete::newline)(body)?;
    let inputs: Vec<String> = iline.split(' ').map(|v| v.to_string()).skip(1).collect();

    let (bi, oline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let outputs: Vec<String> = oline.split(' ').map(|v| v.to_string()).skip(1).collect();

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

    mods.push(Module {
        name: name.to_string(),
        inputs: inputs,
        outputs: outputs,
        luts: luts,
        subckts: subckts,
        gates: gates,
        latches: latches
    });

    if i.len() > body_end_marker.to_string().len() {
        // Advance to the next .end
        (i, _) = take_until(".")(i)?;
    } else {
        // End of file
        (i, _) = take_until("\n")(i)?;
    }

    Ok((i, ""))
}

fn blif_parser<'a>(input: &'a str, modules: &mut Vec<Module>) -> IResult<&'a str, &'a str> {
    // remove comment
    let (i, _) = value((), pair(tag("#"), is_not("\n")))(input)?;
    let (i, _) = take_until(".")(i)?;

    let mut i = i;
    while i.len() > 4 {
        (i, _) = module_body_parser(i, modules)?;
        (i, _) = take_until_or_end("\n.model", i)?;
        (i, _) = terminated_newline(i)?;
    }

    Ok(("", ""))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let blif_file = fs::read_to_string(file_path)?;

    let mut modules = vec![];
    let res = blif_parser(&blif_file, &mut modules);
    match res {
        Ok(_) => {
            println!("Parsing blif file succeeded");
        }
        Err(err) => {
            println!("blif file parsing error:\n{}", err);
        }
    }

// println!("modules\n{:?}", modules);

    Ok(())
}



use std::{
    cmp::Ordering,
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
struct Module {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
    luts: Vec<Lut>,
    subckts: Vec<Subckt>
}

fn lut_table_parser<'a>(input: &'a str, table: &mut Vec<Vec<u8>>) -> IResult<&'a str, &'a str> {
    let mut i = input;
    let mut li = "";
    let mut te = ' ';
    while i.len() > 0 {
        (i, li) = terminated(take_until("\n"), nom::character::complete::newline)(i)?;

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
    let (i, ioline)  = terminated(take_until("\n"), nom::character::complete::newline)(input)?;
    let mut io: Vec<&str> = ioline.split(' ').collect();

    let output = io.pop().unwrap_or("INVALID_OUTPUT").to_string();
    let inputs: Vec<String> = io.iter().map(|v| v.to_string()).collect();
    let (i, table) = take_until(".")(i)?;

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
    let (i, name) = terminated(take_while(|c:char| c.is_alphabetic()),
                               nom::character::complete::multispace0)(input)?;
    let (i, sline) = terminated(take_until("\n"), nom::character::complete::newline)(i)?;

    let mut conns = HashMap::new();
    let conns_vec: Vec<&str> = sline.split(' ').collect();
    conns_vec.iter().for_each(|c| {
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

fn module_body_parser<'a>(input: &'a str, mods: &mut Vec<Module>) -> IResult<&'a str, &'a str> {
    let (i, _) = tag(".model ")(input)?;
    let (i, name) = terminated(take_while(|c:char| c.is_alphabetic()),
                               nom::character::complete::newline)(i)?;
    let (mut i, body) = take_until("end")(i)?;

    let (bi, _) = tag(".inputs ")(body)?;
    let (bi, iline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let inputs: Vec<String> = iline.split(' ').map(|v| v.to_string()).collect();

    let (bi, _) = tag(".outputs ")(bi)?;
    let (bi, oline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let outputs: Vec<String> = oline.split(' ').map(|v| v.to_string()).collect();

    let mut luts = vec![];
    let mut subckts = vec![];
    let mut bi = bi;
    let mut tagstr = "";

    while bi.len() > 1 {
        (bi, tagstr) = terminated(take_until(" "), nom::character::complete::multispace0)(bi)?;
        if tagstr.eq(".names") {
            (bi, _) = lut_body_parser(bi, &mut luts)?;
        } else if tagstr.eq(".subckt") {
            (bi, _) = subckt_parser(bi, &mut subckts)?;
        }
    }

    mods.push(Module {
        name: name.to_string(),
        inputs: inputs,
        outputs: outputs,
        luts: luts,
        subckts: subckts
    });

    if i.len() > 4 {
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

    println!("modules\n{:?}", modules);

    Ok(())
}

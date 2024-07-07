

use std::collections::HashMap;

use nom::{
    bytes::complete::{is_not, tag, take_until, take_while, take_until1},
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
struct Module {
    name: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
    luts: Vec<Lut>
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
    let (i, _) = tag(".names ")(input)?;
    let (i, ioline)  = terminated(take_until("\n"), nom::character::complete::newline)(i)?;
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

fn module_body_parser<'a>(input: &'a str, mods: &mut Vec<Module>) -> IResult<&'a str, &'a str> {
    let (i, _) = tag(".model ")(input)?;
    let (i, name) = terminated(take_while(|c:char| c.is_alphabetic()),
                               nom::character::complete::newline)(i)?;
    let (i, body) = take_until("end")(i)?;

    let (bi, _) = tag(".inputs ")(body)?;
    let (bi, iline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let inputs: Vec<String> = iline.split(' ').map(|v| v.to_string()).collect();

    let (bi, _) = tag(".outputs ")(bi)?;
    let (bi, oline) = terminated(take_until("\n"), nom::character::complete::newline)(bi)?;
    let outputs: Vec<String> = oline.split(' ').map(|v| v.to_string()).collect();

    let mut luts = vec![];
    let mut bi = bi;
    while bi.len() > 1 {
        (bi, _) = lut_body_parser(bi, &mut luts)?;
    }

    mods.push(Module {
        name: name.to_string(),
        inputs: inputs,
        outputs: outputs,
        luts: luts
    });

    Ok((i, ""))
}

fn blif_parser<'a>(input: &'a str, modules: &mut Vec<Module>) -> IResult<&'a str, &'a str> {
    // remove comment
    let (i, _) = value((), pair(tag("#"), is_not(".")))(input)?;

    let mut i = i;
    while i.len() > 3 {
        (i, _) = module_body_parser(i, modules)?;
    }

    Ok(("", ""))
}

fn main() {
    let input_string = "# some comment\n\
                        \n\
                        .model Adder\n\
                        .inputs clock reset io_a[0] io_a[1] io_b[0] io_b[1]\n\
                        .outputs io_c[0] io_c[1]\n\
                        .names $false\n\
                        .names $true\n\
                        1\n\
                        .names $undef\n\
                        .names $abc$2314$io_b[1] $abc$2314$io_a[1] $abc$2314$new_n9_ $abc$2314$io_c[1]\n\
                        001 1\n\
                        010 1\n\
                        100 1\n\
                        111 1\n\
                        .names $abc$2314$io_b[0] $abc$2314$io_a[0] $abc$2314$new_n9_\n\
                        11 1\n\
                        .names $abc$2314$io_b[0] $abc$2314$io_a[0] $abc$2314$io_c[0]\n\
                        01 1\n\
                        10 1\n\
                        .names io_c[0] $techmap$add$Adder.sv:10$940.$auto$alumacc.cc:485:replace_alu$2253.X[0]\n\
                        1 1\n\
                        .names $abc$2314$io_c[1] io_c[1]\n\
                        1 1\n\
                        .names io_a[0] $abc$2314$io_a[0]\n\
                        1 1\n\
                        .names io_b[0] $abc$2314$io_b[0]\n\
                        1 1\n\
                        .names $abc$2314$io_c[0] io_c[0]\n\
                        1 1\n\
                        .names io_a[1] $abc$2314$io_a[1]\n\
                        1 1\n\
                        .names io_b[1] $abc$2314$io_b[1]\n\
                        1 1\n\
                        .end";

    let mut modules = vec![];
    let res = blif_parser(input_string, &mut modules);
    match res {
        Ok(_) => {
            println!("Parsing blif file succeeded");
        }
        Err(err) => {
            println!("blif file parsing error:\n{}", err);
        }
    }

    println!("modules\n{:?}", modules);
}

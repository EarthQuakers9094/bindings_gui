use std::str::FromStr;

use anyhow::{Context, Result};
use bumpalo::Bump;
use chumsky::{
    error::Simple,
    extra,
    pratt::{infix, left},
    prelude::just,
    recursive,
    regex::regex,
    Parser,
};
use egui::{TextEdit, Ui};

#[derive(Debug, PartialEq)]
enum Ast<'a> {
    Add(&'a Ast<'a>, &'a Ast<'a>),
    Mult(&'a Ast<'a>, &'a Ast<'a>),
    Div(&'a Ast<'a>, &'a Ast<'a>),
    Sub(&'a Ast<'a>, &'a Ast<'a>),
    Num(f64),
}

impl<'a> Ast<'a> {
    fn add(arena: &'a Bump, a: &'a Self, b: &'a Self) -> &'a Self {
        arena.alloc(Ast::Add(a, b))
    }
    fn mult(arena: &'a Bump, a: &'a Self, b: &'a Self) -> &'a Self {
        arena.alloc(Ast::Mult(a, b))
    }
    fn div(arena: &'a Bump, a: &'a Self, b: &'a Self) -> &'a Self {
        arena.alloc(Ast::Div(a, b))
    }
    fn sub(arena: &'a Bump, a: &'a Self, b: &'a Self) -> &'a Self {
        arena.alloc(Ast::Sub(a, b))
    }
    fn num(arena: &'a Bump, a: f64) -> &'a Self {
        arena.alloc(Ast::Num(a))
    }
}

fn parse<'a>(s: &'a str, arena: &'a Bump) -> Result<&'a Ast<'a>> {
    let res = recursive::recursive::<_, _, extra::Err<Simple<char>>, _, _>(|a| {
        let atom = regex("-?\\d+(\\.\\d*)?")
            .map(|a| Ok::<_, <f64 as FromStr>::Err>(Ast::num(arena, str::parse::<f64>(a)?)))
            .unwrapped()
            .padded()
            .or(a.delimited_by(just('(').padded(), just(')').padded()));

        let op = |c| just(c).padded();

        atom.pratt((
            infix(left(2), op('*'), |l, _, r, _| Ast::mult(arena, l, r)),
            infix(left(2), op('/'), |l, _, r, _| Ast::div(arena, l, r)),
            infix(left(1), op('+'), |l, _, r, _| Ast::add(arena, l, r)),
            infix(left(1), op('-'), |l, _, r, _| Ast::sub(arena, l, r)),
        ))
    })
    .parse(s);

    Ok(res.output().with_context(|| "failed to parse")?)
}

fn eval(a: &Ast<'_>) -> f64 {
    match a {
        Ast::Add(ast, ast1) => eval(ast) + eval(ast1),
        Ast::Mult(ast, ast1) => eval(ast) * eval(ast1),
        Ast::Div(ast, ast1) => eval(ast) / eval(ast1),
        Ast::Sub(ast, ast1) => eval(ast) - eval(ast1),
        Ast::Num(n) => *n,
    }
}

pub trait NumberInput {
    fn from_f64(a: f64) -> Self;
}

impl NumberInput for f64 {
    fn from_f64(a: f64) -> Self {
        a
    }
}

impl NumberInput for i64 {
    fn from_f64(a: f64) -> Self {
        a.floor() as i64
    }
}

pub fn number_input<N>(text: &mut String, value: &mut N, arena: &Bump, ui: &mut Ui) -> bool
where
    N: ToString + Copy + NumberInput,
{
    let mut update = false;

    let before = arena.alloc_str(text);

    if ui.add(TextEdit::singleline(text)).lost_focus() {
        *value = match parse(text, arena) {
            Ok(ast) => NumberInput::from_f64(eval(ast)),
            Err(_) => *value,
        };

        update = true;

        *text = value.to_string();
    }

    if before != text {
        *value = match parse(text, arena) {
            Ok(ast) => NumberInput::from_f64(eval(ast)),
            Err(_) => *value,
        };

        update = true;
    }

    update
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn precedence() {
        let arena = Bump::new();

        assert_eq!(
            parse("2 * 3 + 1", &arena).unwrap(),
            Ast::add(
                &arena,
                Ast::mult(&arena, Ast::num(&arena, 2.0), Ast::num(&arena, 3.0)),
                Ast::num(&arena, 1.0)
            )
        )
    }

    #[test]
    fn minus() {
        let arena = Bump::new();

        assert_eq!(parse("-2.1", &arena).unwrap(), Ast::num(&arena, -2.1))
    }

    #[test]
    fn lefttoright() {
        let arena = Bump::new();

        assert_eq!(
            parse("2 / 3 * 2", &arena).unwrap(),
            Ast::mult(
                &arena,
                Ast::div(&arena, Ast::num(&arena, 2.0), Ast::num(&arena, 3.0)),
                Ast::num(&arena, 2.0)
            )
        )
    }

    #[test]
    fn lefttorightadd() {
        let arena = Bump::new();

        assert_eq!(
            parse("2 + 3 - 2", &arena).unwrap(),
            Ast::sub(
                &arena,
                Ast::add(&arena, Ast::num(&arena, 2.0), Ast::num(&arena, 3.0)),
                Ast::num(&arena, 2.0)
            )
        )
    }

    #[test]
    fn checkingimnotstupid() {
        assert_eq!(0.01.to_string(), "0.01")
    }

    #[test]
    fn dumb_error() {
        let text = "2.11";
        let arena = Bump::new();

        let value = match parse(&text, &arena) {
            Ok(ast) => NumberInput::from_f64(eval(ast)),
            Err(_) => 0.0,
        };

        let t2 = value.to_string();

        assert_eq!(text, t2)
    }

    #[test]
    fn parens() {
        let arena = Bump::new();

        assert_eq!(
            parse("2.1 * (3 + 5)", &arena).unwrap(),
            Ast::mult(
                &arena,
                Ast::num(&arena, 2.1),
                Ast::add(&arena, Ast::num(&arena, 3.0), Ast::num(&arena, 5.0))
            )
        )
    }

    #[test]
    fn whitespace() {
        let arena = Bump::new();

        assert_eq!(
            parse("       2.1*(3+5)      ", &arena).unwrap(),
            Ast::mult(
                &arena,
                Ast::num(&arena, 2.1),
                Ast::add(&arena, Ast::num(&arena, 3.0), Ast::num(&arena, 5.0))
            )
        )
    }
}

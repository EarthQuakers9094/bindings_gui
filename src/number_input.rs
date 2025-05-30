use anyhow::Result;
use bumpalo::Bump;
use egui::{TextEdit, Ui};
use pom::utf8::{call, end, is_a, sym, Parser};

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

fn whitespace<'a>() -> Parser<'a, ()> {
    is_a(|a| a.is_whitespace()).repeat(0..).discard()
}

fn number<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (is_a(|a| a.is_numeric()).repeat(1..) * (sym('.') * is_a(|a| a.is_numeric()).repeat(0..)).opt())
        .collect()
        .map(|s| -> &'a Ast<'a> { Ast::num(arena, s.parse::<f64>().unwrap()) })
        - whitespace()
}

fn basic<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    number(arena) | ((sym('(') + whitespace()) * call(|| add(arena)) - (sym(')') + whitespace()))
}

enum Operator {
    Add,
    Sub,
    Mult,
    Div,
}

fn plus_operator<'a>() -> Parser<'a, Operator> {
    (sym('+') - whitespace()).map(|_| Operator::Add)
        | (sym('-') - whitespace()).map(|_| Operator::Sub)
}

fn mult_operator<'a>() -> Parser<'a, Operator> {
    (sym('*') - whitespace()).map(|_| Operator::Mult)
        | (sym('/') - whitespace()).map(|_| Operator::Div)
}

fn operator_to_expr<'a>(
    op: Operator,
    a1: &'a Ast<'a>,
    a2: &'a Ast<'a>,
    arena: &'a Bump,
) -> &'a Ast<'a> {
    match op {
        Operator::Add => Ast::add(arena, a1, a2),
        Operator::Sub => Ast::sub(arena, a1, a2),
        Operator::Mult => Ast::mult(arena, a1, a2),
        Operator::Div => Ast::div(arena, a1, a2),
    }
}

fn add<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (mult(arena) + (plus_operator() + call(|| add(arena))).opt()).map(|(a, a2)| {
        if let Some((o, a2)) = a2 {
            operator_to_expr(o, a, a2, arena)
        } else {
            a
        }
    })
}

fn mult<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (basic(arena) + (mult_operator() + call(|| mult(arena))).opt()).map(|(a, a2)| {
        if let Some((o, a2)) = a2 {
            operator_to_expr(o, a, a2, arena)
        } else {
            a
        }
    })
}

fn parse<'a>(s: &'a str, arena: &'a Bump) -> Result<&'a Ast<'a>> {
    Ok((whitespace() * add(arena) - end()).parse(s.as_bytes())?)
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

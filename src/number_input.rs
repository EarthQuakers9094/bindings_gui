use anyhow::Result;
use bumpalo::Bump;
use egui::{Id, TextEdit, Ui};
use pom::utf8::{call, is_a, sym, Parser};

enum Ast<'a> {
    Add(&'a Ast<'a>, &'a Ast<'a>),
    Mult(&'a Ast<'a>, &'a Ast<'a>),
    Div(&'a Ast<'a>, &'a Ast<'a>),
    Sub(&'a Ast<'a>, &'a Ast<'a>),
    Num(f64),
}

fn whitespace<'a>() -> Parser<'a, ()> {
    is_a(|a| a.is_whitespace()).repeat(0..).discard()
}

fn number<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (is_a(|a| a.is_numeric()).repeat(1..) * sym('.') * is_a(|a| a.is_numeric()))
        .collect()
        .map(|s| -> &'a Ast<'a> { arena.alloc(Ast::Num(s.parse::<f64>().unwrap())) })
        - whitespace()
}

fn basic<'a>(arena: &'a Bump) -> Parser<&'a Ast<'a>> {
    number(arena) | (sym('(') * call(|| mult(arena)) - sym(')'))
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
    (sym('+') - whitespace()).map(|_| Operator::Add)
        | (sym('-') - whitespace()).map(|_| Operator::Sub)
}

fn operator_to_expr<'a>(
    op: Operator,
    a1: &'a Ast<'a>,
    a2: &'a Ast<'a>,
    arena: &'a Bump,
) -> &'a Ast<'a> {
    match op {
        Operator::Add => arena.alloc(Ast::Add(a1, a2)),
        Operator::Sub => arena.alloc(Ast::Sub(a1, a2)),
        Operator::Mult => arena.alloc(Ast::Mult(a1, a2)),
        Operator::Div => arena.alloc(Ast::Div(a1, a2)),
    }
}

fn add<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (basic(arena) + (plus_operator() + call(|| add(arena))).opt()).map(|(a, a2)| {
        if let Some((o, a2)) = a2 {
            operator_to_expr(o, a, a2, arena)
        } else {
            a
        }
    })
}

fn mult<'a>(arena: &'a Bump) -> Parser<'a, &'a Ast<'a>> {
    (basic(arena) + (mult_operator() + call(|| add(arena))).opt()).map(|(a, a2)| {
        if let Some((o, a2)) = a2 {
            operator_to_expr(o, a, a2, arena)
        } else {
            a
        }
    })
}

fn parse<'a>(s: &'a str, arena: &'a Bump) -> Result<&'a Ast<'a>> {
    Ok(mult(arena).parse(s.as_bytes())?)
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

pub fn number_input<N>(text: &mut String, value: &mut N, arena: &Bump, ui: &mut Ui)
where
    N: ToString + Copy + NumberInput,
{
    if ui.add(TextEdit::singleline(text)).lost_focus() {
        *value = match parse(&text, arena) {
            Ok(ast) => {
                NumberInput::from_f64(eval(ast))
            },
            Err(_) => *value,
        };

        *text = value.to_string();
    }
}

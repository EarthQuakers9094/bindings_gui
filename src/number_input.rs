use std::str::FromStr;

use anyhow::{Context, Result};
use bumpalo::Bump;
use chumsky::{
    error::Simple,
    extra,
    pratt::{infix, left},
    prelude::{empty, just},
    recursive,
    regex::regex,
    Parser,
};
use egui::{Color32, TextEdit, Ui};

#[derive(Debug, PartialEq)]
enum Ast<'a> {
    Add(&'a Ast<'a>, &'a Ast<'a>),
    Mult(&'a Ast<'a>, &'a Ast<'a>),
    Div(&'a Ast<'a>, &'a Ast<'a>),
    Sub(&'a Ast<'a>, &'a Ast<'a>),
    Num(f64),

    Meters(f64),
    Inches(f64),
    Centimeters(f64),
    Feet(f64),

    Radians(f64),
    Degrees(f64),
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

fn to_constructor(con: &dyn Fn(f64) -> Ast<'static>) -> &dyn Fn(f64) -> Ast<'static> {
    con
}

fn parse<'a>(s: &'a str, arena: &'a Bump) -> Result<&'a Ast<'a>> {
    let res = recursive::recursive::<_, &'a Ast<'a>, extra::Err<Simple<char>>, _, _>(|a| {
        let num = regex("-?\\d+(\\.\\d*)?")
            .map(|a| Ok::<_, <f64 as FromStr>::Err>(str::parse::<f64>(a)?))
            .unwrapped()
            .padded();

        let unit = (just("m")
            .to(to_constructor(&Ast::Meters))
            .or(just("cm").to(to_constructor(&Ast::Centimeters)))
            .or(just("ft").to(to_constructor(&Ast::Feet)))
            .or(just("in").to(to_constructor(&Ast::Inches)))
            .or(just("rad").to(to_constructor(&Ast::Radians)))
            .or(just("deg").to(to_constructor(&Ast::Degrees))))
        .padded();

        let atom = num
            .then(unit.map(Option::Some).or(empty().to(None)))
            .map(
                |(a, b): (f64, Option<&dyn Fn(f64) -> Ast<'static>>)| match b {
                    Some(f) => arena.alloc(f(a)),
                    None => Ast::num(arena, a),
                },
            )
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

enum NumOrUnit {
    Num(f64),
    Distance(f64), // meters
    Angle(f64),    // radians
}

fn eval(a: &Ast<'_>) -> Option<NumOrUnit> {
    match a {
        Ast::Add(ast, ast1) => match (eval(ast)?, eval(ast1)?) {
            (NumOrUnit::Angle(a), NumOrUnit::Angle(b)) => Some(NumOrUnit::Angle(a + b)),
            (NumOrUnit::Distance(a), NumOrUnit::Distance(b)) => Some(NumOrUnit::Distance(a + b)),
            (NumOrUnit::Num(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Num(a + b)),
            (_, _) => None,
        },
        Ast::Mult(ast, ast1) => match (eval(ast)?, eval(ast1)?) {
            (NumOrUnit::Num(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Num(a * b)),
            (NumOrUnit::Num(a), NumOrUnit::Distance(b)) => Some(NumOrUnit::Distance(a * b)),
            (NumOrUnit::Num(a), NumOrUnit::Angle(b)) => Some(NumOrUnit::Angle(a * b)),
            (NumOrUnit::Distance(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Distance(a * b)),
            (NumOrUnit::Angle(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Angle(a * b)),
            (_, _) => None,
        },

        Ast::Div(ast, ast1) => match (eval(ast)?, eval(ast1)?) {
            (NumOrUnit::Num(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Num(a / b)),
            (NumOrUnit::Distance(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Distance(a / b)),
            (NumOrUnit::Distance(a), NumOrUnit::Distance(b)) => Some(NumOrUnit::Num(a / b)),
            (NumOrUnit::Angle(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Angle(a / b)),
            (NumOrUnit::Angle(a), NumOrUnit::Angle(b)) => Some(NumOrUnit::Num(a / b)),
            (_, _) => None,
        },

        Ast::Sub(ast, ast1) => match (eval(ast)?, eval(ast1)?) {
            (NumOrUnit::Angle(a), NumOrUnit::Angle(b)) => Some(NumOrUnit::Angle(a - b)),
            (NumOrUnit::Distance(a), NumOrUnit::Distance(b)) => Some(NumOrUnit::Distance(a - b)),
            (NumOrUnit::Num(a), NumOrUnit::Num(b)) => Some(NumOrUnit::Num(a - b)),
            (_, _) => None,
        },
        Ast::Num(n) => Some(NumOrUnit::Num(*n)),
        Ast::Meters(meters) => Some(NumOrUnit::Distance(*meters)),
        Ast::Inches(inches) => Some(NumOrUnit::Distance(inches * 0.0254)),
        Ast::Centimeters(cm) => Some(NumOrUnit::Distance(cm * 0.01)),
        Ast::Feet(feet) => Some(NumOrUnit::Distance(feet * 12.0 * 0.0254)),
        Ast::Radians(rad) => Some(NumOrUnit::Angle(*rad)),
        Ast::Degrees(degrees) => Some(NumOrUnit::Angle(*degrees / 180.0 * std::f64::consts::PI)),
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

// add error reporting
pub fn number_input<N>(text: &mut String, value: &mut N, arena: &Bump, ui: &mut Ui) -> bool
where
    N: ToString + Copy + NumberInput,
{
    let mut update = false;

    let before = arena.alloc_str(text);

    let mut show_error = false;

    if ui.add(TextEdit::singleline(text)).lost_focus() {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Num(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;

        *text = value.to_string();
    }

    if before != text {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Num(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;
    }

    if show_error {
        ui.colored_label(
            Color32::from_rgb(0xf3, 0x8b, 0xa8),
            "ERROR FAILED TO EVALUATE",
        );
    }

    update
}

// add error reporting
pub fn distance_input<N>(text: &mut String, value: &mut N, arena: &Bump, ui: &mut Ui) -> bool
where
    N: ToString + Copy + NumberInput,
{
    let mut update = false;

    let before = arena.alloc_str(text);

    let mut show_error = false;

    if ui.add(TextEdit::singleline(text)).lost_focus() {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Distance(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;

        *text = format!("{} m", value.to_string());
    }

    if before != text {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Distance(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;
    }

    if show_error {
        ui.colored_label(
            Color32::from_rgb(0xf3, 0x8b, 0xa8),
            "ERROR FAILED TO EVALUATE",
        );
    }

    update
}

// add error reporting
pub fn angle_input<N>(text: &mut String, value: &mut N, arena: &Bump, ui: &mut Ui) -> bool
where
    N: ToString + Copy + NumberInput,
{
    let mut update = false;

    let before = arena.alloc_str(text);

    let mut show_error = false;

    if ui.add(TextEdit::singleline(text)).lost_focus() {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Angle(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;

        *text = format!("{} deg", value.to_string());
    }

    if before != text {
        *value = match parse(text, arena) {
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Angle(n)) => NumberInput::from_f64(n),
                _ => {
                    show_error = true;
                    *value
                }
            },
            Err(_) => {
                show_error = true;
                *value
            }
        };

        update = true;
    }

    if show_error {
        ui.colored_label(
            Color32::from_rgb(0xf3, 0x8b, 0xa8),
            "ERROR FAILED TO EVALUATE",
        );
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
            Ok(ast) => match eval(ast) {
                Some(NumOrUnit::Num(n)) => n,
                _ => 0.0,
            },
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

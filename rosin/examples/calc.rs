#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    style: Arc<Stylesheet>,
    display: DynLabel,
    accumulator: f64,
    register: f64,
    mode: Mode,
    operation: Option<Op>,
}

enum Mode {
    Entry,
    DecimalEntry(u32),
    Result,
}

enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

enum Btn {
    Digit(u8),
    Op(Op),
    Clear,
    Sign,
    Decimal,
    Equals,
}

impl State {
    fn press(&mut self, button: Btn) -> Phase {
        match button {
            Btn::Digit(val) => {
                let mut precision = 0;
                match self.mode {
                    Mode::Entry => {
                        self.register *= 10.0;
                        self.register += val as f64;
                    }
                    Mode::DecimalEntry(exponent) => {
                        self.register += val as f64 / (10f64.powf(exponent as f64));
                        self.mode = Mode::DecimalEntry(exponent + 1);
                        precision = exponent;
                    }
                    Mode::Result => {
                        self.register = val as f64;
                        self.mode = Mode::Entry;
                    }
                }
                self.display.set_text(&format!("{:.*}", precision as usize, self.register))
            }
            Btn::Op(op) => {
                if let Some(prev_op) = &self.operation {
                    match prev_op {
                        Op::Add => self.accumulator += self.register,
                        Op::Sub => self.accumulator -= self.register,
                        Op::Mul => self.accumulator *= self.register,
                        Op::Div => self.accumulator /= self.register,
                    }
                } else {
                    self.accumulator = self.register
                }
                self.operation = Some(op);
                self.register = 0.0;
                self.mode = Mode::Entry;
                Phase::Draw
            }
            Btn::Clear => {
                self.mode = Mode::Entry;
                self.operation = None;
                self.accumulator = 0.0;
                self.register = 0.0;
                self.display.set_text("0")
            }
            Btn::Sign => match self.mode {
                Mode::Entry => {
                    self.register *= -1.0;
                    self.display.set_text(&self.register.to_string())
                }
                Mode::DecimalEntry(precision) => {
                    self.register *= -1.0;
                    self.display.set_text(&format!("{:.*}", precision as usize, self.register))
                }
                Mode::Result => {
                    self.accumulator *= -1.0;
                    self.display.set_text(&self.accumulator.to_string())
                }
            },
            Btn::Decimal => {
                if let Mode::Result = self.mode {
                    self.accumulator = 0.0;
                    self.register = 0.0;
                }
                self.mode = Mode::DecimalEntry(1);
                Phase::Draw
            }
            Btn::Equals => {
                if let Some(prev_op) = &self.operation {
                    match prev_op {
                        Op::Add => self.accumulator += self.register,
                        Op::Sub => self.accumulator -= self.register,
                        Op::Mul => self.accumulator *= self.register,
                        Op::Div => self.accumulator /= self.register,
                    }
                } else {
                    self.accumulator = self.register
                }
                self.mode = Mode::Result;
                self.display.set_text(&self.accumulator.to_string())
            }
        }
    }
}

#[no_mangle]
pub fn main_view(state: &State) -> Node<State, WindowHandle> {
    ui!(state.style.clone(), "root" [
        "display" (state.display.view())
        "row" [
            "btn double" (button("Clear", |s: &mut State, _| { s.press(Btn::Clear) }))
            "btn"        (button("±", |s: &mut State, _| { s.press(Btn::Sign) }))
            "btn orange" (button("÷", |s: &mut State, _| { s.press(Btn::Op(Op::Div)) }))
        ]
        "row" [
            "btn"        (button("7", |s: &mut State, _| { s.press(Btn::Digit(7)) }))
            "btn"        (button("8", |s: &mut State, _| { s.press(Btn::Digit(8)) }))
            "btn"        (button("9", |s: &mut State, _| { s.press(Btn::Digit(9)) }))
            "btn orange" (button("×", |s: &mut State, _| { s.press(Btn::Op(Op::Mul)) }))
        ]
        "row" [
            "btn"        (button("4", |s: &mut State, _| { s.press(Btn::Digit(4)) }))
            "btn"        (button("5", |s: &mut State, _| { s.press(Btn::Digit(5)) }))
            "btn"        (button("6", |s: &mut State, _| { s.press(Btn::Digit(6)) }))
            "btn orange" (button("−", |s: &mut State, _| { s.press(Btn::Op(Op::Sub)) }))
        ]
        "row" [
            "btn"        (button("1", |s: &mut State, _| { s.press(Btn::Digit(1)) }))
            "btn"        (button("2", |s: &mut State, _| { s.press(Btn::Digit(2)) }))
            "btn"        (button("3", |s: &mut State, _| { s.press(Btn::Digit(3)) }))
            "btn orange" (button("+", |s: &mut State, _| { s.press(Btn::Op(Op::Add)) }))
        ]
        "row" [
            "btn double" (button("0", |s: &mut State, _| { s.press(Btn::Digit(0)) }))
            "btn"        (button(".", |s: &mut State, _| { s.press(Btn::Decimal) }))
            "btn orange" (button("=", |s: &mut State, _| { s.press(Btn::Equals) }))
        ]
    ])
}

#[rustfmt::skip]
fn main() {
    let view = new_view!(main_view);

    let window = WindowDesc::new(view)
        .with_title("Rosin Calculator")
        .with_size(400.0, 550.0);

    let mut rl = ResourceLoader::default();

    let state = State {
        style: load_css!(rl, "examples/calc.css"),
        display: DynLabel::new("0"),
        accumulator: 0.0,
        register: 0.0,
        mode: Mode::Entry,
        operation: None,
    };

    AppLauncher::new(rl, window)
        .run(state)
        .expect("Failed to launch");
}

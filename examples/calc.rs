#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

pub struct State {
    display: DynLabel<State>,
    accumulator: f64,
    register: f64,
    mode: Mode,
    operation: Option<Op>,
}

enum Mode {
    Entry,
    DecimalEntry(f64),
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
    Decimal,
    Equals,
}

impl State {
    fn press(&mut self, button: Btn) -> Stage {
        match button {
            Btn::Digit(val) => {
                match self.mode {
                    Mode::Entry => {
                        self.register *= 10.0;
                        self.register += val as f64;
                    }
                    Mode::DecimalEntry(place) => {
                        self.register += val as f64 / (10.0 * place);
                        self.mode = Mode::DecimalEntry(place + 1.0);
                    }
                    Mode::Result => {
                        self.register = val as f64;
                        self.mode = Mode::Entry;
                    }
                }
                self.display.set_text(&self.register.to_string());
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
            }
            Btn::Clear => {
                self.mode = Mode::Entry;
                self.operation = None;
                self.accumulator = 0.0;
                self.register = 0.0;
                self.display.set_text("0");
            }
            Btn::Decimal => {
                self.mode = Mode::DecimalEntry(1.0);
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
                self.display.set_text(&self.accumulator.to_string());
            }
        }

        Stage::Draw
    }
}

pub fn main_view(state: &State) -> Node<State> {
    ui!("root" [
        "display" (state.display.view())
        "row" [
            "btn triple" (button("Clear", |state: &mut State, _| { state.press(Btn::Clear) }))
            "btn orange" (button("/", |state: &mut State, _| { state.press(Btn::Op(Op::Div)) }))
        ]
        "row" [
            "btn"        (button("7", |state: &mut State, _| { state.press(Btn::Digit(7)) }))
            "btn"        (button("8", |state: &mut State, _| { state.press(Btn::Digit(8)) }))
            "btn"        (button("9", |state: &mut State, _| { state.press(Btn::Digit(9)) }))
            "btn orange" (button("x", |state: &mut State, _| { state.press(Btn::Op(Op::Mul)) }))
        ]
        "row" [
            "btn"        (button("4", |state: &mut State, _| { state.press(Btn::Digit(4)) }))
            "btn"        (button("5", |state: &mut State, _| { state.press(Btn::Digit(5)) }))
            "btn"        (button("6", |state: &mut State, _| { state.press(Btn::Digit(6)) }))
            "btn orange" (button("-", |state: &mut State, _| { state.press(Btn::Op(Op::Sub)) }))
        ]
        "row" [
            "btn"        (button("1", |state: &mut State, _| { state.press(Btn::Digit(1)) }))
            "btn"        (button("2", |state: &mut State, _| { state.press(Btn::Digit(2)) }))
            "btn"        (button("3", |state: &mut State, _| { state.press(Btn::Digit(3)) }))
            "btn orange" (button("+", |state: &mut State, _| { state.press(Btn::Op(Op::Add)) }))
        ]
        "row" [
            "btn double" (button("0", |state: &mut State, _| { state.press(Btn::Digit(0)) }))
            "btn"        (button(".", |state: &mut State, _| { state.press(Btn::Decimal) }))
            "btn orange" (button("=", |state: &mut State, _| { state.press(Btn::Equals) }))
        ]
    ])
}

fn main() {
    let state = State {
        display: DynLabel::new(lens!(State => display), "0"),
        accumulator: 0.0,
        register: 0.0,
        mode: Mode::Entry,
        operation: None,
    };

    let view = new_view!(main_view);
    let stylesheet = new_style!("examples/calc.css");
    let window = WindowDesc::new(view).with_title("Rosin Calculator").with_size(650.0, 650.0);

    AppLauncher::default()
        .use_style(stylesheet)
        .add_window(window)
        .add_font_bytes(0, include_bytes!("fonts/Roboto-Regular.ttf"))
        .run(state)
        .expect("Failed to launch");
}

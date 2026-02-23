#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::prelude::*;
use rosin::widgets::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    DecimalEntry(u32),
    Entry,
    Error,
    Result,
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug)]
struct State {
    style: Stylesheet,
    display: Var<f64>,
    accumulator: Option<f64>,
    register: Option<f64>,
    operation: Option<Op>,
    mode: Mode,
}

impl Default for State {
    fn default() -> Self {
        Self {
            style: stylesheet!("examples/styles/calc.css"),
            display: Var::new(0.0),
            accumulator: None,
            register: None,
            operation: None,
            mode: Mode::Entry,
        }
    }
}

impl State {
    pub fn clear(&mut self) {
        self.display.set(0.0);
        self.accumulator = None;
        self.register = None;
        self.operation = None;
        self.mode = Mode::Entry;
    }

    pub fn sign(&mut self) {
        if self.mode == Mode::Error {
            return;
        }

        if let Some(register) = &mut self.register {
            *register = -*register;
            self.display.set(*register);
        } else if let Some(accumulator) = &mut self.accumulator {
            *accumulator = -*accumulator;
            self.display.set(*accumulator);
        }
    }

    pub fn decimal(&mut self) {
        match self.mode {
            Mode::Error => return,
            Mode::Entry => {
                self.mode = Mode::DecimalEntry(1);
            }
            Mode::DecimalEntry(_) => {}
            Mode::Result => {
                self.mode = Mode::DecimalEntry(1);
                self.accumulator = None;
                self.register = None;
                self.display.set(0.0);
            }
        }
    }

    pub fn digit(&mut self, digit: u8) {
        match self.mode {
            Mode::Error => return,
            Mode::Entry => {
                self.accumulator = Some((self.accumulator.unwrap_or(0.0) * 10.0) + digit as f64);
            }
            Mode::DecimalEntry(ref mut precision) => {
                if *precision < 10 {
                    self.accumulator = Some(self.accumulator.unwrap_or(0.0) + (digit as f64 / 10_u32.pow(*precision) as f64));
                    *precision += 1;
                }
            }
            Mode::Result => {
                self.accumulator = Some(digit as f64);
                self.register = None;
                self.operation = None;
                self.mode = Mode::Entry;
            }
        }
        if let Some(accumulator) = self.accumulator {
            self.display.set(accumulator);
        }
    }

    pub fn operation(&mut self, op: Op) {
        match self.mode {
            Mode::Error => return,
            Mode::Result => {}
            Mode::Entry | Mode::DecimalEntry(_) => {
                if self.accumulator.is_some() {
                    if self.register.is_some() {
                        self.calculate();
                        self.accumulator = self.register;
                    }
                    self.register = self.accumulator;
                }
            }
        }
        self.mode = Mode::Entry;
        self.accumulator = None;
        self.operation = Some(op);
    }

    pub fn equals(&mut self) {
        if self.mode == Mode::Error || self.register.is_none() || self.accumulator.is_none() {
            return;
        }

        self.mode = Mode::Result;
        self.calculate();
    }

    /// Panics if register or accumulator are None.
    fn calculate(&mut self) {
        let Some(op) = self.operation else { return };
        let result = match op {
            Op::Add => self.register.unwrap() + self.accumulator.unwrap(),
            Op::Sub => self.register.unwrap() - self.accumulator.unwrap(),
            Op::Mul => self.register.unwrap() * self.accumulator.unwrap(),
            Op::Div => {
                if self.accumulator.unwrap() != 0.0 {
                    self.register.unwrap() / self.accumulator.unwrap()
                } else {
                    self.mode = Mode::Error;
                    f64::NAN
                }
            }
        };
        self.register = Some(result);
        self.display.set(result);
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node()
        .id(id!())
        .classes("root")
        .style_sheet(&state.style)
        .event(On::Create, |_, ctx| ctx.platform().set_menu(Some(MenuDesc::new())))
        .children(|ui| {
            label(ui, id!(), state.display.downgrade()).classes("display");

            ui.node().id(id!()).classes("row").children(|ui| {
                button(ui, id!(), "Clear", |s, _| s.clear()).classes("wide");
                button(ui, id!(), "±", |s, _| s.sign());
                button(ui, id!(), "÷", |s, _| s.operation(Op::Div)).classes("orange");
            });

            ui.node().id(id!()).classes("row").children(|ui| {
                button(ui, id!(), "7", |s, _| s.digit(7));
                button(ui, id!(), "8", |s, _| s.digit(8));
                button(ui, id!(), "9", |s, _| s.digit(9));
                button(ui, id!(), "×", |s, _| s.operation(Op::Mul)).classes("orange");
            });

            ui.node().id(id!()).classes("row").children(|ui| {
                button(ui, id!(), "4", |s, _| s.digit(4));
                button(ui, id!(), "5", |s, _| s.digit(5));
                button(ui, id!(), "6", |s, _| s.digit(6));
                button(ui, id!(), "−", |s, _| s.operation(Op::Sub)).classes("orange");
            });

            ui.node().id(id!()).classes("row").children(|ui| {
                button(ui, id!(), "1", |s, _| s.digit(1));
                button(ui, id!(), "2", |s, _| s.digit(2));
                button(ui, id!(), "3", |s, _| s.digit(3));
                button(ui, id!(), "+", |s, _| s.operation(Op::Add)).classes("orange");
            });

            ui.node().id(id!()).classes("row").children(|ui| {
                button(ui, id!(), "0", |s, _| s.digit(0)).classes("wide");
                button(ui, id!(), ".", |s, _| s.decimal());
                button(ui, id!(), "=", |s, _| s.equals()).classes("orange");
            });
        });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    let window = WindowDesc::new(callback!(main_view))
        .title("Calculator Example")
        .size(300, 400)
        .min_size(200, 300);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

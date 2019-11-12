#[macro_use]
extern crate rosin;
use rosin::prelude::*;

#[derive(Debug)]
pub enum Action {
    Add,
    Subtract,
    Multiply,
    Divide,
    Point,
    Clear,
}

#[derive(Debug)]
pub struct Store {
    pub result: Option<f32>,
    pub action: Action,
    pub integer: Option<u32>,
    pub fraction: Option<Vec<u8>>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            result: None,
            action: Action::Clear,
            integer: None,
            fraction: None,
        }
    }

    pub fn display(&self) -> String {
        if let Some(input) = self.convert_input() {
            input.to_string()
        } else if let Some(result) = self.result {
            result.to_string()
        } else {
            0.to_string()
        }
    }

    pub fn convert_input(&self) -> Option<f32> {
        let mut input = None;
        if let Some(integer) = self.integer {
            input = Some(integer as f32);
        }

        if let Some(digits) = &self.fraction {
            let mut fraction = 0.0;
            for (i, d) in digits.iter().enumerate() {
                fraction += *d as f32 / ((i + 1) as f32 * 10.0);
            }

            if let Some(whole) = input {
                input = Some(whole + fraction);
            } else {
                input = Some(fraction);
            }
        }

        input
    }
}

fn calculate(store: &mut Store) -> Redraw {
    if let Some(input) = store.convert_input() {
        if let Some(result) = store.result {
            match store.action {
                Action::Add => store.result = Some(result + input),
                Action::Subtract => store.result = Some(result - input),
                Action::Multiply => store.result = Some(result * input),
                Action::Divide => store.result = Some(result / input),
                _ => {}
            }
        } else {
            store.result = Some(input);
        }
        store.action = Action::Clear;
        store.integer = None;
        store.fraction = None;
    }

    Redraw::Yes
}

fn press_number(store: &mut Store, number: u8) -> Redraw {
    if let Some(fraction) = &mut store.fraction {
        fraction.push(number);
    } else if let Some(integer) = store.integer {
        store.integer = Some(integer * 10 + number as u32);
    } else {
        store.integer = Some(number as u32);
    }

    Redraw::Yes
}

fn press_action(store: &mut Store, action: Action) -> Redraw {
    match action {
        Action::Clear => {
            *store = Store::new();
        }
        Action::Point => {
            store.fraction = Some(Vec::new());
        }
        _ => {
            calculate(store);
            store.action = action;
        }
    }

    Redraw::Yes
}

#[no_mangle]
pub fn main_view(store: &Store) -> Dom<Store> {
    dom! {
        Wrap [
            display [.label(store.display())]

            btn_triple, orange [^button!("Clear", |s| press_action(s, Action::Clear))]
            btn, orange [^button!("/", |s| press_action(s, Action::Divide))]

            btn [^button!("7", |s| press_number(s, 7))]
            btn [^button!("8", |s| press_number(s, 8))]
            btn [^button!("9", |s| press_number(s, 9))]
            btn, orange [^button!("x", |s| press_action(s, Action::Multiply))]

            btn [^button!("4", |s| press_number(s, 4))]
            btn [^button!("5", |s| press_number(s, 5))]
            btn [^button!("6", |s| press_number(s, 6))]
            btn, orange [^button!("-", |s| press_action(s, Action::Subtract))]

            btn [^button!("1", |s| press_number(s, 1))]
            btn [^button!("2", |s| press_number(s, 2))]
            btn [^button!("3", |s| press_number(s, 3))]
            btn, orange [^button!("+", |s| press_action(s, Action::Add))]

            btn_double [^button!("0", |s| press_number(s, 0))]
            btn [^button!(".", |s| press_action(s, Action::Point))]
            btn, orange [^button!("=", |s| calculate(s))]
        ]
    }
}

#[macro_export]
macro_rules! button {
    ($label:expr, |$s:ident, $a:ident| $($action:tt)*) => {
        Dom::div()
            .label($label)
            .event(On::Click,
                |$s, $a| {
                    $($action)*
                }
            )
    };
    ($label:expr, |$s:ident| $($action:tt)*) => {
        Dom::div()
            .label($label)
            .event(On::Click,
                |$s, _| {
                    $($action)*
                }
            )
    };
    ($label:expr, $handler:ident) => {
        Dom::div()
            .label($label)
            .event(On::Click, $handler)
    };
}

#[derive(Debug, Default)]
pub struct AnimButton {
    pub transition: f32,
}

#[macro_export]
macro_rules! anim_button {
    ($type:ty [ $store:ident $($path:tt)* ], $label:expr, |$s:ident, $a:ident| $($action:tt)*) => {
        Dom::div()
            .label($label)
            .event(On::Hover,
                |store: &mut $type, _app: &mut App<$type>| {
                    store$($path)*.transition = 1.0;
                    Redraw::Yes
                })
            .event(On::Click,
                |$s, $a| {
                    $($action)*
                }
            )
    };
}

#[derive(Debug, Default)]
pub struct TextBox {
    pub active: bool,
    pub value: String,
}

#[macro_export]
macro_rules! textbox {
    ($type:ty [ $store:ident $($path:tt)* ]) => {
        Dom::div()
            .class("textbox")
            .event(On::Click,
                |s: &mut $type, _a: &mut App<$type>| {
                    store$($path)*.active = true;
                    Redraw::Yes
                }
            )
    }
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rosin::{keyboard_types::NamedKey, prelude::*, widgets::*};

struct State {
    url: Var<String>,
    textbox: TextBox,
}

impl Default for State {
    fn default() -> Self {
        Self {
            url: Var::new("https://www.wikipedia.com/".into()),
            textbox: TextBox::default(),
        }
    }
}

impl State {
    fn go(&self, ctx: &EventCtx<'_, WindowHandle>) {
        ctx.platform().open_url(&self.url.get());
    }
}

fn main_view(state: &State, ui: &mut Ui<State, WindowHandle>) {
    ui.node()
        .id(id!())
        .style_sheet(dark_theme())
        .classes("root")
        .event(On::PointerDown, |_, ctx| {
            ctx.set_focus(None);
        })
        // When the user presses tab, cycle to the next focusable node.
        // TODO - this should be baked into widgets
        .event(On::Keyboard, move |_, ctx| {
            let Some(ev) = ctx.keyboard() else { return };
            if ev.state == KeyState::Down && ev.key == Key::Named(NamedKey::Tab) {
                if ev.modifiers.shift() {
                    ctx.focus_previous();
                } else {
                    ctx.focus_next();
                }
            }
        })
        .children(|ui| {
            label(ui, id!(), "Enter a URL");
            state.textbox.view(ui, id!(), *state.url).event(On::Keyboard, |s, ctx| {
                let Some(ev) = ctx.keyboard() else { return };
                if ev.state == KeyState::Down && ev.key == Key::Named(NamedKey::Enter) {
                    s.go(ctx);
                }
            });
            button(ui, id!(), "Go!", |s, ctx| s.go(ctx));
        });
}

#[rustfmt::skip]
fn main() {
    env_logger::init();

    // This is required to support keyboard shortcuts for copy, paste, etc.
    let edit_menu = MenuDesc::new()
        .add_item(MenuItem::Standard(StandardAction::Cut))
        .add_item(MenuItem::Standard(StandardAction::Copy))
        .add_item(MenuItem::Standard(StandardAction::Paste))
        .add_separator()
        .add_item(MenuItem::Standard(StandardAction::SelectAll));
    let main_menu = MenuDesc::new()
        .add_item(MenuItem::Submenu {
            title: LocalizedStringBuilder::new("Application").build(), 
            menu: MenuDesc::new(),
            enabled: true,
        })
        .add_item(MenuItem::Submenu {
            title: LocalizedStringBuilder::new("Edit").build(),
            menu: edit_menu,
            enabled: true,
        });

    let window = WindowDesc::new(callback!(main_view))
        .menu(main_menu)
        .title("URL Example")
        .size(400, 150)
        .min_size(250, 150)
        .max_size(600, 150);

    AppLauncher::new(window)
        .run(State::default(), TranslationMap::default())
        .expect("Failed to launch");
}

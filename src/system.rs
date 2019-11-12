#![allow(dead_code)]

use glutin::{
    Context, ContextCurrentState, ContextError, NotCurrent, PossiblyCurrent, WindowedContext,
};
use takeable_option::Takeable;
use webrender::api::*;

pub struct Notifier {
    events_proxy: glutin::EventsLoopProxy,
}

impl Notifier {
    pub fn new(events_proxy: glutin::EventsLoopProxy) -> Notifier {
        Notifier { events_proxy }
    }
}

impl RenderNotifier for Notifier {
    fn clone(&self) -> Box<dyn RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        let _ = self.events_proxy.wakeup();
    }

    fn new_frame_ready(
        &self,
        _: DocumentId,
        _scrolled: bool,
        _composite_needed: bool,
        _render_time: Option<u64>,
    ) {
        self.wake_up();
    }
}

pub enum ContextWrapper<T: ContextCurrentState> {
    Headless(Context<T>),
    Windowed(WindowedContext<T>),
}

impl<T: ContextCurrentState> ContextWrapper<T> {
    pub fn headless(&mut self) -> &mut Context<T> {
        match self {
            ContextWrapper::Headless(ref mut ctx) => ctx,
            _ => panic!(),
        }
    }

    pub fn windowed(&mut self) -> &mut WindowedContext<T> {
        match self {
            ContextWrapper::Windowed(ref mut ctx) => ctx,
            _ => panic!(),
        }
    }

    fn map<T2: ContextCurrentState, FH, FW>(
        self,
        fh: FH,
        fw: FW,
    ) -> Result<ContextWrapper<T2>, (Self, ContextError)>
    where
        FH: FnOnce(Context<T>) -> Result<Context<T2>, (Context<T>, ContextError)>,
        FW: FnOnce(
            WindowedContext<T>,
        ) -> Result<WindowedContext<T2>, (WindowedContext<T>, ContextError)>,
    {
        match self {
            ContextWrapper::Headless(ctx) => match fh(ctx) {
                Ok(ctx) => Ok(ContextWrapper::Headless(ctx)),
                Err((ctx, err)) => Err((ContextWrapper::Headless(ctx), err)),
            },
            ContextWrapper::Windowed(ctx) => match fw(ctx) {
                Ok(ctx) => Ok(ContextWrapper::Windowed(ctx)),
                Err((ctx, err)) => Err((ContextWrapper::Windowed(ctx), err)),
            },
        }
    }
}

pub enum ContextCurrentWrapper {
    PossiblyCurrent(ContextWrapper<PossiblyCurrent>),
    NotCurrent(ContextWrapper<NotCurrent>),
}

impl ContextCurrentWrapper {
    fn map_possibly<F>(self, f: F) -> Result<Self, (Self, ContextError)>
    where
        F: FnOnce(
            ContextWrapper<PossiblyCurrent>,
        ) -> Result<
            ContextWrapper<NotCurrent>,
            (ContextWrapper<PossiblyCurrent>, ContextError),
        >,
    {
        match self {
            ret @ ContextCurrentWrapper::NotCurrent(_) => Ok(ret),
            ContextCurrentWrapper::PossiblyCurrent(ctx) => match f(ctx) {
                Ok(ctx) => Ok(ContextCurrentWrapper::NotCurrent(ctx)),
                Err((ctx, err)) => Err((ContextCurrentWrapper::PossiblyCurrent(ctx), err)),
            },
        }
    }

    fn map_not<F>(self, f: F) -> Result<Self, (Self, ContextError)>
    where
        F: FnOnce(
            ContextWrapper<NotCurrent>,
        ) -> Result<
            ContextWrapper<PossiblyCurrent>,
            (ContextWrapper<NotCurrent>, ContextError),
        >,
    {
        match self {
            ret @ ContextCurrentWrapper::PossiblyCurrent(_) => Ok(ret),
            ContextCurrentWrapper::NotCurrent(ctx) => match f(ctx) {
                Ok(ctx) => Ok(ContextCurrentWrapper::PossiblyCurrent(ctx)),
                Err((ctx, err)) => Err((ContextCurrentWrapper::NotCurrent(ctx), err)),
            },
        }
    }
}

pub type ContextId = usize;
#[derive(Default)]
pub struct ContextManager {
    current: Option<ContextId>,
    others: Vec<(ContextId, Takeable<ContextCurrentWrapper>)>,
    next_id: ContextId,
}

impl ContextManager {
    pub fn insert(&mut self, ctx: ContextCurrentWrapper) -> ContextId {
        let id = self.next_id;
        self.next_id += 1;

        if let ContextCurrentWrapper::PossiblyCurrent(_) = ctx {
            if let Some(old_current) = self.current {
                unsafe {
                    self.modify(old_current, |ctx| {
                        ctx.map_possibly(|ctx| {
                            ctx.map(
                                |ctx| Ok(ctx.treat_as_not_current()),
                                |ctx| Ok(ctx.treat_as_not_current()),
                            )
                        })
                    })
                    .unwrap()
                }
            }
            self.current = Some(id);
        }

        self.others.push((id, Takeable::new(ctx)));
        id
    }

    pub fn remove(&mut self, id: ContextId) -> ContextCurrentWrapper {
        if Some(id) == self.current {
            self.current.take();
        }

        let this_index = self
            .others
            .binary_search_by(|(sid, _)| sid.cmp(&id))
            .unwrap();
        Takeable::take(&mut self.others.remove(this_index).1)
    }

    fn modify<F>(&mut self, id: ContextId, f: F) -> Result<(), ContextError>
    where
        F: FnOnce(
            ContextCurrentWrapper,
        ) -> Result<ContextCurrentWrapper, (ContextCurrentWrapper, ContextError)>,
    {
        let this_index = self
            .others
            .binary_search_by(|(sid, _)| sid.cmp(&id))
            .unwrap();

        let this_context = Takeable::take(&mut self.others[this_index].1);

        match f(this_context) {
            Err((ctx, err)) => {
                self.others[this_index].1 = Takeable::new(ctx);
                Err(err)
            }
            Ok(ctx) => {
                self.others[this_index].1 = Takeable::new(ctx);
                Ok(())
            }
        }
    }

    pub fn get_current(
        &mut self,
        id: ContextId,
    ) -> Result<&mut ContextWrapper<PossiblyCurrent>, ContextError> {
        unsafe {
            let this_index = self
                .others
                .binary_search_by(|(sid, _)| sid.cmp(&id))
                .unwrap();
            if Some(id) != self.current {
                let old_current = self.current.take();

                if let Err(err) = self.modify(id, |ctx| {
                    ctx.map_not(|ctx| ctx.map(|ctx| ctx.make_current(), |ctx| ctx.make_current()))
                }) {
                    // something went wrong, make sure no context is current
                    if let Some(old_current) = old_current {
                        if let Err(err2) = self.modify(old_current, |ctx| {
                            ctx.map_possibly(|ctx| {
                                ctx.map(|ctx| ctx.make_not_current(), |ctx| ctx.make_not_current())
                            })
                        }) {
                            panic!(
                                "[Rosin] Couldn't `make_current` or `make_not_current`, {:?}, {:?}",
                                err, err2
                            );
                        }
                    }

                    if let Err(err2) = self.modify(id, |ctx| {
                        ctx.map_possibly(|ctx| {
                            ctx.map(|ctx| ctx.make_not_current(), |ctx| ctx.make_not_current())
                        })
                    }) {
                        panic!(
                            "[Rosin] Couldn't `make_current` or `make_not_current`, {:?}, {:?}",
                            err, err2
                        );
                    }

                    return Err(err);
                }

                self.current = Some(id);

                if let Some(old_current) = old_current {
                    self.modify(old_current, |ctx| {
                        ctx.map_possibly(|ctx| {
                            ctx.map(
                                |ctx| Ok(ctx.treat_as_not_current()),
                                |ctx| Ok(ctx.treat_as_not_current()),
                            )
                        })
                    })
                    .unwrap();
                }
            }

            match *self.others[this_index].1 {
                ContextCurrentWrapper::PossiblyCurrent(ref mut ctx) => Ok(ctx),
                ContextCurrentWrapper::NotCurrent(_) => panic!(),
            }
        }
    }
}

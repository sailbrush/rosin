// **WIN32 THREAD SAFETY**
//
// **Presumebly** most if not all win32 ops are thread-safe unless specified otherwise
// I CAN NOT find an official source for this but I have been told this several times
// I will for not forfit on trying to find an answer and simply assume this is the case
// if this is proven wrong I am sorry, I believe simualted thread safety can be done though using
// a manual queue (maybe with a VecDeque within eather the ThreadLockedView or AppLauncher) and with
// a manual block (this one I dunno how to impl since I gave it 0 tought as of now)
//
// This does not mean I can guarantee safety though, so I personally do not think this deserves to be
// within a stable version of eather Rosin nor Sailbrush, in my opinion at least.
// 
// Why am I not implementing my own safety system? Because I'd a lil scared of it being slow + it's
// easyer to do it this wai for the meantime. I tried making an abstraction that is similar to how
// it's done on the MacOs port tho + should be easely changable to a custom made thread safety system
//
// TL;DR the safety guarantee as of now is "trust me bro" and I'm sorry

// TODO:
//  - Queueing & Blocking on thread
//      - Make a noop abstraction layer because the functions **may** be thread safe
//      - I need to be able to remember the queue per thread
//      - I would need probably some use of static?
//      - Have an inbetween abstraction layer between what `WindowHandle` does and what `RosinView` does?
//  - Rendering & Drawing
//  - (eventually) Try to remove as much dependency on unsafe as possible

pub mod app;
pub mod handle;

mod proc_fn;
mod view;

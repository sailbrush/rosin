//! Types related to event handling and dispatch.

use std::panic::Location;
use std::path::PathBuf;
use std::time::Duration;

use accesskit::ActionRequest;
use keyboard_types::KeyboardEvent;
use kurbo::{Point, Rect, RoundedRect, Size, Vec2};
use parley::{AlignmentOptions, Layout};
use vello::Scene;

use crate::prelude::*;
use crate::{layout, text};

/// Summary of the results of an event dispatch cycle.
///
/// This is returned to the platform after dispatching events.
#[derive(Debug, Default, Copy, Clone)]
pub struct DispatchInfo {
    /// The total number of callbacks that were executed.
    pub callback_count: u32,

    /// Whether event propagation was stopped by an event handler calling [`EventCtx::stop_propagation`].
    pub bubbling_stopped: bool,

    /// Whether a request to close the window was intercepted and cancelled by a handler.
    pub stop_window_close: bool,
}

/// The list of events that Nodes can register callbacks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On {
    /// This event is fired when AccessKit requests a semantic action on the node.
    AccessibilityAction,

    /// This event is fired once per display refresh, unless the node is disabled.
    AnimationFrame,

    /// This event is fired when a node loses focus.
    Blur,

    /// This event is fired when a callback requests change propagation from [`EventCtx::emit_change`], or when
    /// a change is queued by the platform, such as when the text input handler modifies a text field.
    ///
    /// If a change event is sent to a node without an `On::Change` handler,
    /// it will instead be queued on the nearest ancestor that does have one, if it exists.
    Change,

    /// This event is fired when an application command is triggered, such as selecting
    /// an item in the main menu, a context menu, or clicking a modal dialog button.
    ///
    /// Commands from the main menu will always be sent to the root node.
    Command,

    /// This event is fired every time a node is added to the tree, even when it's disabled.
    ///
    /// If a node doesn't have an id, it will be treated as a new node and fire
    /// every time the tree is rebuilt, which probably isn't what you want.
    Create,

    /// This event is fired every time a node is removed from the tree, even when it's disabled.
    ///
    /// If a node doesn't have an id, it will be treated as a new node and fire
    /// every time the tree is rebuilt, which probably isn't what you want.
    Destroy,

    /// This event is fired by the platform after a file dialog closes.
    FileDialog,

    /// This event is fired when a node gains focus.
    ///
    /// Nodes with handlers for [`On::Focus`] are considered "focusable" and can be focused by
    /// methods such as [`EventCtx::focus_next`] and [`EventCtx::focus_previous`].
    Focus,

    /// This event is fired when a keyboard key is pressed or released, unless the keypress was handled by an IME handler.
    ///
    /// Sent to the focused node (if any) and always to the root node.
    Keyboard,

    /// This event is fired when a pointer button is pressed while the pointer is over this node, or any of its descendants,
    /// unless one of the descendants called `stop_propagation()` when handling the event.
    ///
    /// The event targets the frontmost node under the pointer first, then bubbles up
    /// through its parents to the root, firing on every node along the way.
    PointerDown,

    /// This event is fired when the pointer first enters the node, or any of its descendants, unless the pointer is captured.
    PointerEnter,

    /// This event is fired when the pointer leaves the node, and all of its descendants, unless the pointer is captured.
    PointerLeave,

    /// This event is fired when the pointer moves while inside the node or any of its descendants,
    /// unless one of the descendants called `stop_propagation()` when handling the event, or the pointer is captured.
    ///
    /// The event targets the frontmost node under the pointer first, then bubbles up
    /// through its parents to the root, firing on every node along the way.
    PointerMove,

    /// This event is fired when a pointer button is released while over the node or any of its descendants,
    /// unless one of the descendants called `stop_propagation()` when handling the event, or the pointer is captured.
    ///
    /// The event targets the frontmost node under the pointer first, then bubbles up
    /// through its parents to the root, firing on every node along the way.
    PointerUp,

    /// This event is fired when the pointer wheel scrolls while the pointer is over the node or any of its descendants,
    /// unless one of the descendants called `stop_propagation()` when handling the event, or the pointer is captured.
    ///
    /// The event targets the frontmost node under the pointer first, then bubbles up
    /// through its parents to the root, firing on every node along the way.
    PointerWheel,

    /// This event is fired by the platform after a specified delay when a timer is requested.
    Timer,

    /// This event is fired on the root node when the window loses focus.
    WindowBlur,

    /// This event is fired on the root node when a window close is requested.
    ///
    /// If the callback calls [`EventCtx::stop_window_close`], the window will be prevented from closing.
    WindowClose,

    /// This event is fired on the root node when the window gains focus.
    WindowFocus,
}

impl On {
    /// Returns `true` if this event is a pointer event.
    pub fn is_pointer(&self) -> bool {
        matches!(self, On::PointerDown | On::PointerUp | On::PointerMove | On::PointerEnter | On::PointerLeave | On::PointerWheel)
    }
}

/// A context provided to the [`on_measure`](crate::tree::Ui::on_measure) callback of a node.
pub struct MeasureCtx<'a> {
    pub style: &'a Style,
    pub max_size: Option<Size>,
}

/// A context provided to the [`on_canvas`](crate::tree::Ui::on_canvas) callback of a node for rendering.
///
/// `CanvasCtx` provides access to the Vello [`Scene`] for issuing graphics commands in local space,
/// as well as the node's layout information, computed styles, and interaction state (active/focused).
pub struct CanvasCtx<'a> {
    pub did_layout: bool,
    pub is_active: bool,
    pub is_enabled: bool,
    pub is_focused: bool,
    pub perf_info: &'a PerfInfo,
    pub rect: &'a RoundedRect,
    pub scene: &'a mut Scene,
    pub style: &'a Style,
    pub translation_map: TranslationMap,
}

impl<'a> CanvasCtx<'a> {
    /// Returns the rect just inside the borders of the node in local space. Doesn't account for rounded corners.
    #[inline]
    pub fn padding_box(&self) -> Rect {
        layout::padding_box(self.style, self.rect)
    }

    /// Returns the maximum width content could be before overflowing node.
    #[inline]
    pub fn max_content_width(&self) -> f32 {
        layout::max_content_width(self.style, self.rect)
    }

    /// Draw text according to this node's CSS styles.
    #[inline]
    pub fn draw_text(&mut self, text: &str) {
        let max_width = layout::max_content_width(self.style, self.rect);
        let mut layout = text::layout_text(&self.style.get_font_layout_style(), Some(max_width), text);
        let origin = layout::align_and_position_text(self.style, self.rect, &mut layout);
        text::draw_text(self.scene, self.style, origin, &layout);
    }

    /// Draw text at the provided location according to this node's CSS styles.
    #[inline]
    pub fn draw_text_at_origin(&mut self, origin: Point, max_width: impl Into<Option<f32>>, text: &str) {
        let max_width = max_width.into();
        let mut layout = text::layout_text(&self.style.get_font_layout_style(), max_width, text);
        layout.align(max_width, self.style.text_align.into(), AlignmentOptions::default());
        text::draw_text(self.scene, self.style, origin, &layout);
    }

    /// Draws a text layout at the specified location.
    #[inline]
    pub fn draw_text_layout(&mut self, origin: Point, layout: &Layout<[u8; 4]>) {
        text::draw_text(self.scene, self.style, origin, layout);
    }
}

/// The result of a file dialog interaction.
#[derive(Clone, Debug)]
pub enum FileDialogResponse {
    /// The user confirmed an Open operation.
    /// Contains the list of selected files or folders.
    Opened(Vec<PathBuf>),
    /// The user confirmed a Save operation.
    /// Contains the destination path.
    Saved(PathBuf),
    /// The user closed the dialog without confirming.
    Cancelled,
}

impl FileDialogResponse {
    /// Returns the first selected path regardless of whether the dialog was Open or Save.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            FileDialogResponse::Opened(paths) => paths.first(),
            FileDialogResponse::Saved(path) => Some(path),
            FileDialogResponse::Cancelled => None,
        }
    }

    /// Returns all selected paths. If this was a Save dialog, it returns a vec with a single path.
    pub fn paths(&self) -> Vec<PathBuf> {
        match self {
            FileDialogResponse::Opened(paths) => paths.clone(),
            FileDialogResponse::Saved(path) => vec![path.clone()],
            FileDialogResponse::Cancelled => Vec::new(),
        }
    }

    /// Returns true if the response came from an Open dialog.
    pub fn is_opened(&self) -> bool {
        matches!(self, FileDialogResponse::Opened { .. })
    }

    /// Returns true if the response came from a Save dialog.
    pub fn is_saved(&self) -> bool {
        matches!(self, FileDialogResponse::Saved { .. })
    }

    /// Returns true if the user cancelled the dialog.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, FileDialogResponse::Cancelled)
    }
}

/// An identifier for a command originating in the platform.
///
/// This is typically associated with items in the main menu, context menus, or buttons on modal dialogs.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u32);

impl From<u32> for CommandId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

/// Event specific payload attached to an [`EventCtx`].
#[derive(Clone, Debug)]
pub enum EventInfo {
    None,
    Pointer(PointerEvent),
    Keyboard(KeyboardEvent),
    Animation(Duration),
    File(FileDialogResponse),
    Command(CommandId),
    AccessibilityAction(ActionRequest),
}

#[derive(Copy, Clone)]
pub(crate) enum FocusDirection {
    None,
    Next,
    Previous,
}

/// Information about how long it took to render the previous frame.
///
/// Event callbacks are not included.
#[derive(Default, Debug, Clone, Copy)]
pub struct PerfInfo {
    /// Number of frames that have been drawn since creating the viewport.
    pub frame_number: u64,

    /// How many nodes are in the tree.
    pub node_count: usize,

    /// How long it took to build the tree last frame.
    pub build_time: Duration,

    // How long it took to apply styles last frame.
    pub style_time: Duration,

    /// How long the layout phase took last frame. This may include a selector matching pass and a second layout pass.
    pub layout_time: Duration,

    /// Set to `true` if layout needed to be calculated twice last frame.
    pub layout_twice: bool,

    /// How long it took to build the Vello scene last frame.
    pub scene_time: Duration,

    /// How long it took to paint the surface last frame.
    ///
    /// Provided by the platform integration through [`Viewport::report_paint_time`].
    pub paint_time: Duration,
}

impl PerfInfo {
    #[inline]
    pub fn total_time(&self) -> Duration {
        self.build_time + self.style_time + self.layout_time + self.scene_time + self.paint_time
    }

    #[inline]
    pub fn cpu_time(&self) -> Duration {
        self.build_time + self.style_time + self.layout_time + self.scene_time
    }

    #[inline]
    pub fn gpu_time(&self) -> Duration {
        self.paint_time
    }
}

/// A context provided to event handlers registered with [`Ui::event`].
///
/// [`EventCtx`] is the primary interface for nodes to respond to user input. It provides:
/// - State Updates: Methods to set focus, capture the pointer, or mark a node as active.
/// - Event Data: Access to specific event details like pointer coordinates or keyboard keys.
/// - Propagation Control: Ability to stop events from bubbling up the tree or to prevent default window actions.
/// - Layout Info: Access to the node's computed size and position.
///
/// It also provides a handle to the platform, allowing the callback
/// to trigger system-level actions like opening URLs or changing the cursor.
pub struct EventCtx<'a, H> {
    pub(crate) active_node: Option<NodeId>,
    pub(crate) captured_node: Option<NodeId>,
    pub(crate) emit_change: bool,
    pub(crate) event_type: On,
    pub(crate) focus_direction: FocusDirection,
    pub(crate) focused_node: Option<NodeId>,
    pub(crate) handle: &'a H,
    pub(crate) id: Option<NodeId>,
    pub(crate) idx: usize,
    pub(crate) info: EventInfo,
    pub(crate) is_enabled: bool,
    pub(crate) perf_info: &'a PerfInfo,
    pub(crate) pointer_delta: Option<Vec2>,
    pub(crate) rect: &'a RoundedRect,
    pub(crate) stop_bubbling: bool,
    pub(crate) stop_window_close: bool,
    pub(crate) style: &'a Style,
    pub(crate) translation_map: TranslationMap,
    pub(crate) viewport_size: Size,
}

impl<'a, H> EventCtx<'a, H> {
    /// Returns a handle provided by the platform integration.
    /// The default platform integration returns a `rosin::handle::WindowHandle`.
    #[inline]
    pub fn platform(&self) -> &H {
        self.handle
    }

    /// Sets the currently active node. This makes `:active` CSS selectors apply to the node. Purely cosmetic.
    ///
    /// You can pass `None` to deactivate.
    #[inline]
    pub fn set_active(&mut self, id: Option<NodeId>) {
        self.active_node = id;
    }

    /// Returns `true` if the current node is active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.id.is_some() && self.id == self.active_node
    }

    /// Returns `true` if the current node is enabled.
    ///
    /// A node's enabled status is controlled by a [`UIParam`] passed to [`Ui::enabled`] when building the tree.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Sets the currently focused node. This makes `:focus` CSS selectors apply to the node, and routes [`On::Keyboard`] events to it.
    ///
    /// You can pass `None` to unfocus.
    #[inline]
    pub fn set_focus(&mut self, id: Option<NodeId>) {
        self.focused_node = id;
    }

    /// Transfers focus to the next focusable node node in the tree.
    /// If nothing is focused, the first focusable node in the tree will gain focus.
    #[inline]
    pub fn focus_next(&mut self) {
        self.focus_direction = FocusDirection::Next;
    }

    /// Transfers focus to the previous focusable node node in the tree.
    /// If nothing is focused, the last focusable node in the tree will gain focus.
    #[inline]
    pub fn focus_previous(&mut self) {
        self.focus_direction = FocusDirection::Previous;
    }

    /// Returns `true` if the current node is focused.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.id.is_some() && self.id == self.focused_node
    }

    /// Returns the size of the total drawable area of the viewport in logical pixels.
    #[inline]
    pub fn viewport_size(&self) -> Size {
        self.viewport_size
    }

    /// Returns the computed style for this node.
    #[inline]
    pub fn style(&self) -> &Style {
        self.style
    }

    /// Returns the final laid-out rectangle of this node.
    #[inline]
    pub fn rect(&self) -> &RoundedRect {
        self.rect
    }

    /// Return the current node's id, if it has one.
    ///
    /// - In debug builds, this will log an error if there is no id for the node.
    #[inline]
    #[track_caller]
    pub fn id(&self) -> Option<NodeId> {
        if cfg!(debug_assertions) && self.id.is_none() {
            // If a handler requests an id, it likely assumes that there is one.
            // This should be loud, otherwise things will fail silently.
            let location = Location::caller();
            log::error!("id() must be called on a node with an id: {location}");
        }
        self.id
    }

    /// Returns the type of event that triggered this callback.
    #[inline]
    pub fn event_type(&self) -> On {
        self.event_type
    }

    /// Returns the change in position between this pointer event and the previous, if available.
    /// The first pointer event fired when the cursor enters the viewport will not have a delta.
    #[inline]
    pub fn pointer_delta(&self) -> Option<Vec2> {
        self.pointer_delta
    }

    /// Returns some information about how long it took to render the previous frame.
    #[inline]
    pub fn perf_info(&self) -> &PerfInfo {
        self.perf_info
    }

    /// Returns the global map of translations.
    #[inline]
    pub fn get_translation_map(&self) -> TranslationMap {
        self.translation_map.clone()
    }

    /// Begins capturing pointer events. When captured, the pointer is treated as if it is always inside the node,
    /// so [`On::PointerEnter`] and [`On::PointerLeave`] events will never fire.
    ///
    /// - In debug builds, this will log an error if there is no id for the node.
    #[inline]
    #[track_caller]
    pub fn begin_pointer_capture(&mut self) {
        if cfg!(debug_assertions) && self.id.is_none() {
            let location = Location::caller();
            log::error!("begin_pointer_capture() must be called on a node with an id: {location}");
        }
        self.captured_node = self.id;
    }

    /// Releases the pointer capture, so other nodes can start receiving pointer events again.
    #[inline]
    pub fn end_pointer_capture(&mut self) {
        self.captured_node = None;
    }

    /// Returns `true` if this node has captured the pointer.
    #[inline]
    pub fn is_pointer_captured(&self) -> bool {
        self.id.is_some() && self.id == self.captured_node
    }

    /// Stop a pointer event from bubbling up to ancestor nodes.
    /// [`On::PointerEnter`] and [`On::PointerLeave`] events do not bubble.
    /// - In debug builds, this will log an error if called from a non-pointer event.
    #[inline]
    #[track_caller]
    pub fn stop_propagation(&mut self) {
        if cfg!(debug_assertions) && !self.event_type.is_pointer() {
            let location = Location::caller();
            log::error!("stop_propagation() must be called from a pointer event: {location}");
        }
        self.stop_bubbling = true;
    }

    /// Requests an [`On::Change`] dispatch after this callback returns.
    ///
    /// The dispatcher will queue [`On::Change`] on this node (if it handles it),
    /// otherwise on the first ancestor with an [`On::Change`] handler.
    /// If the current event is [`On::Change`], it will only look for an ancestor
    /// to avoid re-queuing the same handler.
    #[inline]
    pub fn emit_change(&mut self) {
        self.emit_change = true;
    }

    /// Stops the window from closing.
    ///
    /// - In debug builds, this will log an error if not called from the root node's [`On::WindowClose`] handler.
    #[inline]
    #[track_caller]
    pub fn stop_window_close(&mut self) {
        if cfg!(debug_assertions) {
            let location = Location::caller();
            if self.event_type != On::WindowClose {
                log::error!("stop_window_close() must be called from an On::WindowClose handler: {location}");
            } else if self.idx != 0 {
                log::error!("stop_window_close() must be called from the root node: {location}");
            }
        }
        self.stop_window_close = true;
    }

    /// In pointer event handlers, this returns the complete pointer event info.
    #[inline]
    pub fn pointer(&self) -> Option<&PointerEvent> {
        if let EventInfo::Pointer(event) = &self.info {
            Some(event)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// If available, returns the position of the pointer event relative to the top-left of the current node.
    #[inline]
    pub fn local_pointer_pos(&self) -> Option<Point> {
        if let EventInfo::Pointer(event) = &self.info {
            let vec = event.viewport_pos - self.rect.origin();
            Some(Point::new(vec.x, vec.y))
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// In [`On::Keyboard`] handlers, this returns the keyboard event info.
    #[inline]
    pub fn keyboard(&self) -> Option<&KeyboardEvent> {
        if let EventInfo::Keyboard(event) = &self.info {
            Some(event)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// In [`On::AnimationFrame`] handlers, this returns the duration since the last animation frame.
    #[inline]
    pub fn dt(&self) -> Option<&Duration> {
        if let EventInfo::Animation(dt) = &self.info {
            Some(dt)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// In [`On::FileDialog`] handlers, this returns the information returned by the requested file dialog.
    #[inline]
    pub fn file_dialog_response(&self) -> Option<&FileDialogResponse> {
        if let EventInfo::File(file) = &self.info {
            Some(file)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// In [`On::Command`] handlers, this returns the [`CommandId`] associated with the menu item picked.
    #[inline]
    pub fn command_id(&self) -> Option<CommandId> {
        if let EventInfo::Command(cmd) = &self.info {
            Some(*cmd)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// In [`On::AccessibilityAction`] handlers, this returns the AccessKit action request info.
    #[inline]
    pub fn action_request(&self) -> Option<&ActionRequest> {
        if let EventInfo::AccessibilityAction(req) = &self.info {
            Some(req)
        } else {
            // we don't log an error in case client code wants to route multiple events to the same handler.
            None
        }
    }

    /// Returns the raw event payload for this callback.
    ///
    /// This is the same data accessed by the typed helpers like [`EventCtx::pointer`], [`EventCtx::keyboard`], etc.
    #[inline]
    pub fn info(&self) -> &EventInfo {
        &self.info
    }

    /// Returns the rect just inside the borders of the node. Doesn't account for rounded corners.
    #[inline]
    pub fn padding_box(&self) -> Rect {
        layout::padding_box(self.style, self.rect)
    }

    /// Returns the maximum width content could be before overflowing the node.
    #[inline]
    pub fn max_content_width(&self) -> f32 {
        layout::max_content_width(self.style, self.rect)
    }
}

/// A context provided to the [`Ui::on_accessibility`] callback of a node.
pub struct AccessibilityCtx<'a> {
    /// The node's ID.
    pub id: NodeId,

    /// The node's current text.
    pub text: Option<&'a UIString>,

    /// The global translation map.
    pub translation_map: TranslationMap,

    /// The AccessKit node to mutate role/name/value/actions/state/etc.
    pub node: &'a mut accesskit::Node,
}

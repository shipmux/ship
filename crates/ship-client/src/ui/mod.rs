//! The full-screen client, opened by bare `ship`.

mod agents;
mod attention;
mod draw;
mod keys;
mod lane;
mod memory;
mod menu;
mod mouse;
mod observer;
mod outbox;
mod panes;
mod run;
mod sidebar;
mod terminal;

use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    future::{Future, pending, poll_fn},
    mem,
    path::Path,
    pin::{Pin, pin},
    sync::Arc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{Event, EventStream},
    terminal::size,
};
use futures_util::{Stream, StreamExt};
use ratatui::layout::Rect;
use ship_core::{
    command::{
        Command, Direction, PaneTarget, Run, TabTarget,
        pane::{self, PaneCommand},
        tab::{self, TabCommand},
    },
    geometry::TabGeometry,
    id::{Attachment, IdOf},
    layout::{Axis, Layout},
    model::{NodeId, Pane, Switch, Tab},
    prelude::*,
    protocol::{
        AttachRequest, ClipboardText, EndReason, Ended, InputFrame, KeyInput, MoveTab, MoveTabBody,
        PasteInput, ResizePane, SelectView, SseEvent, ViewInput,
    },
    run::{Ended as RunEnded, OnExit, Runner},
    screen::{Screen, Size},
    tree::{self, Tabs},
};
use ship_macros::traced;
use tokio::{
    sync::mpsc,
    time::{self, MissedTickBehavior},
};

use self::{
    attention::Attention,
    draw::Status,
    keys::{Handled, Keys},
    lane::{Item, Lane, MenuAt, TabMove, Turn},
    memory::Memory,
    menu::{Menu, Picked},
    mouse::{Clicked, Mouse},
    observer::Observer,
    outbox::Outbox,
    sidebar::{Sidebar, movable},
    terminal::TerminalGuard,
};
use crate::{
    Client, KeyScope, Scope,
    keymap::{
        Action, AgentSettings, ClientAction, ClientAgents, ClientPane, ClientSettings, ClientTab,
        ConfigAction, Menus, Sidebar as SidebarAction, TabDirection,
    },
    sound::Speaker,
    watch,
};

const FIRST_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
/// At most one view PUT per frame.
const FRAME: Duration = Duration::from_millis(16);

type Events = Pin<Box<dyn Stream<Item = Result<SseEvent>>>>;
/// One item per save of the config file.
type Saves = Pin<Box<dyn Stream<Item = ()>>>;
/// A request the loop awaits alongside everything else.
type Request<'a, T = ()> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

/// Where a navigation key moves the selection.
#[derive(Clone, Copy)]
enum Step {
    /// Around the visible sidebar rows.
    Tab { forward: bool },
    /// Around the viewed tab's slot fronts in layout order.
    Pane { forward: bool },
    /// To the selected pane's neighbor on that side.
    Focus(Direction),
}

/// What `client.tab.collapse` or `client.tab.expand` does on a tab's row.
enum Hop {
    /// Fold or unfold the row, keeping the selection.
    Fold { collapsed: bool },
    /// Select this tab where `Memory::landing` puts it.
    Select(IdOf<Tab>),
}

/// Why the client stopped.
#[derive(Debug)]
enum Exit {
    Detached,
    ServerStopped,
    Signaled,
}

/// Selection requested when the client first attaches.
pub enum InitialSelection {
    None,
    /// Select the first tab's landing pane when this client started the server.
    FirstPane,
}

/// Bare `ship`. Opens the attach stream at the terminal's tab area, then takes
/// over the terminal. `initial_selection` selects nothing or the first tab's
/// pane when this client launched the server. On
/// every `Attached` it numbers its requests for that attachment. Keys go
/// through the keymap loaded from `config`, or the defaults with the error on
/// the status line, and reloaded when the file is saved; a bad save keeps the
/// running keymap and shows the error. Keys queue unread and are interpreted
/// one at a time, against the state left by earlier input.
/// Keys, clicks and actions share one ordered lane. Server requests wait for
/// their reply and the replica before the next turn; input goes out through
/// the outbox. The frame tick sends area only. A cut connection or a refused
/// number keeps the last screen, drops the lane and the outbox, and
/// reconnects with backoff (250 ms doubling to 5 s).
///
/// Returns Ok after detach or `Ended(ServerShutdown)`, printing why after
/// the terminal is restored. SIGTERM, SIGHUP and SIGINT restore the terminal
/// and return.
pub async fn run(
    client: &Client,
    initial_selection: InitialSelection,
    config: &Path,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<()> {
    let signaled = signaled()?;
    let loaded = match ClientSettings::load(config) {
        Ok(settings) => (settings, None),
        Err(error) => {
            tracing::warn!(config = %config.display(), %error, "config unusable; using defaults");
            (ClientSettings::defaults(), Some(error.to_string()))
        }
    };
    let (cols, rows) = terminal_size()?;
    let area = draw::tab_area(cols, rows, loaded.0.sidebar_width);
    let selection = match initial_selection {
        InitialSelection::FirstPane => navigate(
            &client.tabs().await?,
            &Memory::default(),
            &Sidebar::new(loaded.0.sidebar_width),
            None,
            Step::Tab { forward: true },
        ),
        InitialSelection::None => None,
    };
    let request = AttachRequest {
        selection,
        area: Some(area),
        feed: Default::default(),
    };
    // Fail on an unreachable server before taking the terminal.
    let events = attach_after(client, request.clone(), Duration::ZERO).await?;
    let exit = {
        // Every client line names its connection. Each attach swaps in a new
        // span, which every poll enters, as `.instrument` does with a fixed one.
        let connection = RefCell::new(tracing::info_span!("client"));
        let mut drive = pin!(drive(
            client,
            request,
            events,
            signaled,
            config,
            loaded,
            &connection,
            run_output,
        ));
        poll_fn(|cx| {
            let span = connection.borrow().clone();
            let _entered = span.enter();
            drive.as_mut().poll(cx)
        })
        .await
    }?;
    tracing::info!(?exit, "client exiting");
    match exit {
        Exit::Detached => eprintln!("detached"),
        Exit::ServerStopped => eprintln!("server stopped"),
        Exit::Signaled => {}
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "UI startup inputs and run output"
)]
async fn drive(
    client: &Client,
    mut request: AttachRequest,
    events: Events,
    signaled: impl Future<Output = ()>,
    config: &Path,
    loaded: (ClientSettings, Option<String>),
    connection: &RefCell<tracing::Span>,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<Exit> {
    let guard = &mut TerminalGuard::enter()?;
    tokio::pin!(signaled);
    let mut events = Some(events);
    let mut reconnect: Option<Request<Events>> = None;
    let mut backoff = FIRST_BACKOFF;
    let (settings, message) = loaded;
    guard.set_mouse(settings.mouse)?;
    let (sender, ended) = mpsc::unbounded_channel();
    let mut local = Local {
        observer: Observer::default(),
        memory: Memory::default(),
        sidebar: Sidebar::new(settings.sidebar_width),
        keys: Keys::new(settings.keymap),
        mouse: Mouse::default(),
        overlay: None,
        outbox: Outbox::new(client),
        message,
        attention: Attention::default(),
        speaker: Speaker::start(),
        agents: settings.agents,
        runner: Runner::new(run_output, OnExit::Report(sender)),
        ended,
    };
    let mut saves: Option<Saves> = match watch::changes(config) {
        Ok(saves) => Some(Box::pin(saves)),
        Err(error) => {
            tracing::warn!(config = %config.display(), %error, "cannot watch config; live reload off");
            local.message = local.message.or(Some(error.to_string()));
            None
        }
    };
    let mut lane = Lane::default();
    let mut terminal_events = EventStream::new();
    let mut terminal = terminal_size()?;
    guard.resize(terminal)?;
    let mut putting: Option<Request> = None;
    let mut tick = time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &mut local)?;
    // Set when this connection's work must be abandoned.
    let mut cut = false;
    loop {
        let mut visible = tokio::select! {
            event = next(&mut events) => match event {
                Some(Ok(SseEvent::Ended(Ended { reason: EndReason::ServerShutdown }))) => {
                    return Ok(Exit::ServerStopped);
                }
                Some(Ok(SseEvent::Clipboard(ClipboardText { text }))) => {
                    guard.copy(&text)?;
                    false
                }
                Some(Ok(event)) if !matches!(event, SseEvent::Ended(_)) => {
                    if let SseEvent::Attached(attached) = &event {
                        let span = tracing::info_span!(
                            parent: None,
                            "client",
                            attachment = %attached.attachment
                        );
                        span.in_scope(|| tracing::info!("attached"));
                        connection.replace(span);
                        client.order(Some(attached.attachment));
                        backoff = FIRST_BACKOFF;
                    }
                    let screen = matches!(event, SseEvent::Screen(_));
                    let attached = matches!(event, SseEvent::Attached(_));
                    let previous = local.observer.record().and_then(|record| record.selection);
                    let visible = local.observer.apply(event);
                    if attached {
                        local.attention.attached(&local.observer);
                    } else if !screen {
                        local.attention.observe(&local.observer, Instant::now(), local.agents.delay);
                    }
                    if !screen && let Some(replica) = &local.observer.replica {
                        let selection = local.observer.record().and_then(|record| record.selection);
                        local.memory.observe(&replica.tabs, selection);
                        if previous != selection {
                            local.sidebar.tabs.reveal(&replica.tabs, selection);
                        }
                    }
                    visible
                }
                lost => {
                    match lost {
                        Some(Err(error)) => tracing::warn!(%error, "disconnected; reconnecting"),
                        Some(Ok(SseEvent::Ended(Ended { reason: EndReason::Lost }))) => {
                            tracing::warn!("event stream fell behind; reconnecting");
                        }
                        _ => tracing::warn!("event stream ended without notice; reconnecting"),
                    }
                    cut = true;
                    false
                }
            },
            result = finish(&mut reconnect) => match result {
                Ok(stream) => {
                    reconnect = None;
                    events = Some(stream);
                    false
                }
                Err(error) => {
                    tracing::debug!(%error, ?backoff, "reconnect failed");
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    request.area = Some(view_area(terminal, &local.sidebar));
                    reconnect = Some(Box::pin(attach_after(client, request.clone(), backoff)));
                    false
                }
            },
            result = local.outbox.next() => match result {
                Err(error) if refused(&error) => {
                    tracing::warn!(%error, "input refused; reattaching");
                    cut = true;
                    false
                }
                Err(error) => {
                    local.message = Some(error.to_string());
                    true
                }
                Ok(()) => false,
            },
            result = lane.finished() => {
                match result {
                    Err(error) if refused(&error) => {
                        tracing::warn!(%error, "request refused; reattaching");
                        cut = true;
                    }
                    Err(error) => local.message = Some(error.to_string()),
                    Ok(()) => {}
                }
                true
            }
            Some(ended) = local.ended.recv() => {
                local.message = Some(run::describe(&ended));
                true
            }
            save = next(&mut saves) => {
                match save {
                    Some(()) => {
                        local.message = local.reload(guard, config).err().map(|error| {
                            tracing::warn!(config = %config.display(), %error, "config reload failed");
                            error.to_string()
                        });
                    }
                    None => saves = None,
                }
                true
            }
            result = finish(&mut putting) => {
                putting = None;
                if let Err(error) = result {
                    local.message = Some(error.to_string());
                }
                false
            }
            () = until(local.attention.due()) => {
                for fired in local.attention.fire(&local.observer, Instant::now()) {
                    if let Some(clip) = local.agents.sounds.resolve(fired.app.as_deref(), fired.notice) {
                        local.speaker.play(clip);
                    }
                    if local.agents.popups {
                        guard.notify(&fired.popup())?;
                    }
                }
                false
            }
            event = terminal_events.next() => match event {
                Some(Ok(Event::FocusGained | Event::FocusLost)) => {
                    local.attention.set_focused(matches!(event, Some(Ok(Event::FocusGained))));
                    true
                }
                Some(Ok(Event::Resize(cols, rows))) => {
                    terminal = (cols, rows);
                    guard.resize(terminal)?;
                    local.overlay = None;
                    true
                }
                Some(Ok(Event::Mouse(event))) if local.overlay.is_none() && guard.mouse() && local.observer.connected => {
                    let mut mouse = mem::take(&mut local.mouse);
                    let was_dragging = mouse.dragging();
                    let clicked = mouse.route(event, &local.scene(Rect::new(0, 0, terminal.0, terminal.1)), lane.busy());
                    local.mouse = mouse;
                    let mut visible = was_dragging || local.mouse.dragging();
                    if let Some(clicked) = clicked {
                        match clicked {
                            Clicked::Select(node) => lane.push(Item::Select(node)),
                            Clicked::Fold(tab) => lane.push(Item::Fold(tab)),
                            Clicked::Move(tab) => lane.push(Item::Move(tab)),
                            Clicked::Menu(target, at) => lane.push(Item::Menu(MenuAt { target, at })),
                            Clicked::Frame(frame) => lane.push(Item::Mouse(frame)),
                            Clicked::Resize(positions) => {
                                for position in positions { lane.push(Item::Resize(position)); }
                            }
                            Clicked::Width(width) => {
                                local.sidebar.set_width(width, terminal.0);
                                visible = true;
                            }
                            Clicked::Split(row) => {
                                local.sidebar.set_split(row);
                                visible = true;
                            }
                            Clicked::SidebarScroll { at, scroll } => {
                                local.sidebar.scroll(at, scroll);
                                visible = true;
                            }
                        }
                    }
                    visible
                }
                Some(Ok(Event::Mouse(_))) if local.overlay.is_none() => false,
                Some(Ok(event)) => {
                    if local.observer.connected {
                        lane.push(Item::Event(event));
                        false
                    } else {
                        if let Some(exit) = local.offline(event, guard, config, terminal.0) {
                            return Ok(exit);
                        }
                        true
                    }
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if putting.is_none()
                && local.observer.connected
                && local.observer.caught_up(client.written())
                && unsent(&local.observer, view_area(terminal, &local.sidebar)).is_some() =>
            {
                if let Some((attachment, view)) = unsent(&local.observer, view_area(terminal, &local.sidebar)) {
                    putting = Some(Box::pin(async move {
                        client.set_view(attachment, &view).await.map(|_| ())
                    }));
                }
                false
            }
            () = &mut signaled => return Ok(Exit::Signaled),
        };
        // Keep the screens, but abandon all work from this connection.
        if mem::take(&mut cut) {
            (events, putting) = (None, None);
            lane.clear();
            client.forget();
            if let Some(remembered) = local.cut(view_area(terminal, &local.sidebar)) {
                request = remembered;
            }
            reconnect = Some(Box::pin(attach_after(client, request.clone(), backoff)));
            visible = true;
        }
        while let Some(item) = lane.next(&local.observer, client.written()) {
            // Program input may still be buffered or in flight. Finish it
            // before an action or click can take effect.
            if !local.outbox.is_empty()
                && matches!(
                    item,
                    Item::Action(_)
                        | Item::Select(_)
                        | Item::Fold(_)
                        | Item::Resize(_)
                        | Item::Move(_)
                        | Item::Menu(_)
                )
            {
                lane.first(vec![item]);
                break;
            }
            // Input frames change only the server's screen; redraw on its
            // publication, not on each keystroke or mouse frame.
            visible |= matches!(
                item,
                Item::Action(_) | Item::Select(_) | Item::Fold(_) | Item::Move(_) | Item::Menu(_)
            );
            let interpreting = matches!(item, Item::Event(_));
            let overlay = local.overlay.is_some();
            let mode = local.keys.shown_mode().cloned();
            match local.turn(client, item, Rect::new(0, 0, terminal.0, terminal.1)) {
                Turn::Done => {}
                Turn::Wait(request) => lane.wait(request),
                Turn::First(items) => lane.first(items),
                Turn::Local(action) => {
                    local.message = local
                        .now(action, guard, config, terminal.0)
                        .err()
                        .map(|error| error.to_string());
                    visible = true;
                }
                Turn::Detach => {
                    // Queued input must reach the server before the attachment
                    // ends, otherwise it is lost.
                    while !local.outbox.is_empty() {
                        tokio::select! {
                            result = local.outbox.next() => if let Err(error) = result {
                                tracing::warn!(%error, "input before detaching failed");
                            },
                            () = &mut signaled => return Ok(Exit::Signaled),
                        }
                    }
                    return Ok(Exit::Detached);
                }
            }
            visible |= interpreting
                && (overlay || local.overlay.is_some() || local.keys.shown_mode() != mode.as_ref());
        }
        if let Some(replica) = &local.observer.replica {
            local.sidebar.tabs.reconcile(&replica.tabs);
            local.mouse.reconcile(&replica.tabs);
        }
        if visible {
            redraw(guard, &mut local)?;
        }
    }
}

/// At most one overlay captures input before the mode machine.
enum Overlay {
    Menu(Menu),
}

/// Client-local state used to resolve each input at its turn.
struct Local<'a> {
    observer: Observer,
    memory: Memory,
    sidebar: Sidebar,
    keys: Keys,
    mouse: Mouse,
    overlay: Option<Overlay>,
    outbox: Outbox<'a>,
    message: Option<String>,
    attention: Attention,
    speaker: Speaker,
    /// Replaced whole on reload; readers take the piece they need.
    agents: AgentSettings,
    /// Runs outlive this client, including its terminal.
    runner: Runner,
    ended: mpsc::UnboundedReceiver<RunEnded>,
}

/// Published geometry shared by the router and pane painter.
struct Scene<'a> {
    panes: Rect,
    tabs: Option<&'a Tabs>,
    tab: Option<&'a Tab>,
    geometry: Option<&'a TabGeometry>,
    selected: Option<IdOf<Pane>>,
    screens: &'a HashMap<IdOf<Pane>, Arc<Screen>>,
    sidebar: &'a Sidebar,
    menus: &'a Menus,
}

impl<'a> Local<'a> {
    fn scene(&self, screen: Rect) -> Scene<'_> {
        let selected = self.observer.selected();
        let tab = selected.as_ref().map(|selected| selected.tab);
        Scene {
            panes: draw::split(screen, self.sidebar.width()).1,
            tabs: self.observer.replica.as_ref().map(|replica| &replica.tabs),
            tab,
            geometry: tab.and_then(|tab| tab.geometry.as_deref()),
            selected: selected
                .and_then(|selected| selected.pane)
                .map(|pane| pane.id),
            screens: &self.observer.screens,
            sidebar: &self.sidebar,
            menus: &self.keys.keymap.menus,
        }
    }

    /// Interpret input once, through the overlay or the active mode.
    fn read(&mut self, event: Event) -> Handled {
        let handled = match &mut self.overlay {
            Some(Overlay::Menu(menu)) => match menu.handle(&event) {
                Picked::Run(action) => {
                    self.overlay = None;
                    Handled::Action(action)
                }
                Picked::Close => {
                    self.overlay = None;
                    Handled::None
                }
                Picked::Open => Handled::None,
            },
            None => self.keys.handle(event),
        };
        if matches!(handled, Handled::Action(_)) {
            self.message = None;
        }
        handled
    }

    /// Disconnected input is read immediately and never held for reconnect.
    fn offline(
        &mut self,
        event: Event,
        guard: &mut TerminalGuard,
        config: &Path,
        cols: u16,
    ) -> Option<Exit> {
        match self.read(event) {
            Handled::Action(Action::Client(ClientAction::Detach {})) => {
                return Some(Exit::Detached);
            }
            Handled::Action(Action::Client(
                action @ (ClientAction::Mode(_)
                | ClientAction::Sidebar(_)
                | ClientAction::Config(_)),
            )) => {
                self.message = self
                    .now(action, guard, config, cols)
                    .err()
                    .map(|error| error.to_string());
            }
            _ => {}
        }
        None
    }

    fn now(
        &mut self,
        action: ClientAction,
        guard: &mut TerminalGuard,
        config: &Path,
        cols: u16,
    ) -> Result<()> {
        match action {
            ClientAction::Mode(mode) => self.keys.enter(mode),
            ClientAction::Sidebar(SidebarAction::Toggle {}) => {
                self.sidebar.shown = !self.sidebar.shown;
                Ok(())
            }
            ClientAction::Sidebar(SidebarAction::Resize { direction, amount }) => {
                let amount = amount.unwrap_or(1);
                match direction {
                    Direction::Left | Direction::Right => {
                        self.sidebar.nudge(direction, amount, cols);
                    }
                    Direction::Up | Direction::Down => self.sidebar.nudge_split(direction, amount),
                }
                Ok(())
            }
            ClientAction::Config(ConfigAction::Reload {}) => self.reload(guard, config),
            _ => unreachable!("only mode, sidebar and config actions run locally"),
        }
    }

    /// A bad reload keeps the running settings; a valid one closes overlays
    /// and drops presses whose releases may now go to the outer terminal.
    fn reload(&mut self, guard: &mut TerminalGuard, config: &Path) -> Result<()> {
        let settings = ClientSettings::load(config)?;
        guard.set_mouse(settings.mouse)?;
        self.mouse = Mouse::default();
        self.overlay = None;
        self.sidebar.configure(settings.sidebar_width);
        self.keys.replace(settings.keymap);
        self.agents = settings.agents;
        Ok(())
    }

    #[traced]
    fn turn(&mut self, client: &'a Client, item: Item, screen: Rect) -> Turn<'a> {
        match item {
            Item::Event(event) => match self.read(event) {
                Handled::Action(action) => Turn::First(vec![Item::Action(action)]),
                Handled::Key(key) => Turn::First(vec![Item::Key(key)]),
                Handled::Paste(text) => Turn::First(vec![Item::Paste(text)]),
                Handled::None => Turn::Done,
            },
            Item::Menu(MenuAt { target, at }) => {
                self.overlay = Menu::open(menu::items(&self.keys.keymap.menus, target), at, screen)
                    .map(Overlay::Menu);
                if self.overlay.is_some()
                    && let Some(node) = target.node()
                {
                    return self.land(client, node);
                }
                Turn::Done
            }
            Item::Mouse(_) | Item::Resize(_) if self.overlay.is_some() => Turn::Done,
            Item::Action(action) => self.act(client, action),
            Item::Key(key) => {
                if let Some(pane) = typing_pane(&self.observer, &self.memory) {
                    self.send(InputFrame::Key(KeyInput { pane, key }));
                }
                Turn::Done
            }
            Item::Paste(text) => {
                if let Some(pane) = typing_pane(&self.observer, &self.memory) {
                    self.send(InputFrame::Paste(PasteInput { pane, text }));
                }
                Turn::Done
            }
            Item::Select(node) => {
                self.overlay = None;
                self.land(client, node)
            }
            Item::Fold(tab) => {
                self.overlay = None;
                if self
                    .observer
                    .replica
                    .as_ref()
                    .is_some_and(|replica| tree::tab(&replica.tabs, tab).is_ok())
                {
                    self.sidebar.tabs.toggle(tab);
                }
                Turn::Done
            }
            Item::Mouse(frame) => {
                self.send(frame);
                Turn::Done
            }
            Item::Move(moving) => {
                self.overlay = None;
                if !self
                    .observer
                    .replica
                    .as_ref()
                    .is_some_and(|replica| movable(&replica.tabs, moving.tab, moving.to))
                {
                    return Turn::Done;
                }
                if let MoveTab::Parent(Some(parent)) = moving.to {
                    self.sidebar.tabs.fold(parent, false);
                }
                Turn::Wait(Box::pin(async move {
                    match client
                        .move_tab(moving.tab, &MoveTabBody { at: moving.to })
                        .await
                    {
                        Err(error)
                            if matches!(
                                error.code(),
                                ErrorCode::NotFound | ErrorCode::InvalidStructure
                            ) =>
                        {
                            Ok(())
                        }
                        result => result.map(|_| ()),
                    }
                }))
            }
            Item::Resize(position) => {
                if !self.observer.replica.as_ref().is_some_and(|replica| {
                    tree::pane_owner(&replica.tabs, position.edge.pane).is_ok()
                }) {
                    return Turn::Done;
                }
                Turn::Wait(Box::pin(async move {
                    match client
                        .resize(
                            position.edge.pane,
                            &ResizePane {
                                direction: match position.edge.axis {
                                    Axis::Horizontal => Direction::Right,
                                    Axis::Vertical => Direction::Down,
                                },
                                amount: None,
                                to: Some(position.to),
                            },
                        )
                        .await
                    {
                        Err(error) if matches!(error.code(), ErrorCode::NotFound) => Ok(()),
                        result => result.map(|_| ()),
                    }
                }))
            }
        }
    }

    /// Select a clicked target, using the tab's remembered landing pane.
    fn land(&mut self, client: &'a Client, node: NodeId) -> Turn<'a> {
        let Some(replica) = &self.observer.replica else {
            return Turn::Done;
        };
        if tree::path(&replica.tabs, node).is_none() {
            return Turn::Done;
        }
        let node = match node {
            NodeId::Tab(id) => match tree::tab(&replica.tabs, id) {
                Ok(tab) => self.memory.landing(tab),
                Err(_) => return Turn::Done,
            },
            pane => pane,
        };
        self.select(client, node)
    }

    #[traced]
    fn send(&mut self, frame: InputFrame) {
        self.outbox.push(frame);
    }

    /// Abandon the connection's input and gestures, keeping the screens.
    /// Returns the request to reattach with, when there's a record to keep.
    fn cut(&mut self, area: Size) -> Option<AttachRequest> {
        self.outbox.clear();
        self.mouse = Mouse::default();
        self.overlay = None;
        self.observer.connected = false;
        self.observer.remembered(area)
    }

    fn select(&mut self, client: &'a Client, node: NodeId) -> Turn<'a> {
        let Some(attachment) = self.observer.attachment else {
            return Turn::Done;
        };
        Turn::Wait(Box::pin(async move {
            match client
                .select(
                    attachment,
                    &SelectView {
                        selection: Some(node),
                    },
                )
                .await
            {
                Err(error) if matches!(error.code(), ErrorCode::NotFound) => Ok(()),
                result => result.map(|_| ()),
            }
        }))
    }

    fn act(&mut self, client: &'a Client, action: Action) -> Turn<'a> {
        let selection = self.observer.record().and_then(|record| record.selection);
        if leaves_zoom(&action)
            && let Some(zoomed) = zoomed_pane(&self.observer, selection)
        {
            return Turn::First(vec![
                Item::Action(Action::Server(set_zoom(zoomed, None))),
                Item::Action(action),
            ]);
        }
        let step = match &action {
            Action::Client(ClientAction::Tab(ClientTab::Next {})) => {
                Some(Step::Tab { forward: true })
            }
            Action::Client(ClientAction::Tab(ClientTab::Prev {})) => {
                Some(Step::Tab { forward: false })
            }
            Action::Client(ClientAction::Pane(ClientPane::Next {})) => {
                Some(Step::Pane { forward: true })
            }
            Action::Client(ClientAction::Pane(ClientPane::Prev {})) => {
                Some(Step::Pane { forward: false })
            }
            Action::Client(ClientAction::Pane(ClientPane::Focus { direction })) => {
                Some(Step::Focus(*direction))
            }
            _ => None,
        };
        if let Some(step) = step {
            if let Some(replica) = &self.observer.replica
                && let Some(to) =
                    navigate(&replica.tabs, &self.memory, &self.sidebar, selection, step)
                && Some(to) != selection
            {
                return self.select(client, to);
            }
            return Turn::Done;
        }
        let done = match action {
            Action::Server(command) => {
                let scope = Scope::Keys(KeyScope {
                    attachment: self.observer.attachment,
                    selection,
                    tabs: self
                        .observer
                        .replica
                        .as_ref()
                        .map(|replica| replica.tabs.clone())
                        .unwrap_or_default(),
                });
                return Turn::Wait(Box::pin(async move {
                    client.execute(command, &scope).await.map(|_| ())
                }));
            }
            Action::Client(action) => match action {
                ClientAction::Pane(ClientPane::Swap { direction }) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(NodeId::Pane(from)) = selection
                        && let Some(NodeId::Pane(other)) = navigate(
                            &replica.tabs,
                            &self.memory,
                            &self.sidebar,
                            selection,
                            Step::Focus(direction),
                        )
                    {
                        return Turn::First(vec![Item::Action(Action::Server(Command::Pane(
                            PaneCommand::Swap(pane::Swap {
                                pane: PaneTarget { id: Some(from) },
                                other,
                            }),
                        )))]);
                    }
                    Ok(())
                }
                ClientAction::Pane(ClientPane::Zoom { zoomed }) => {
                    match zoom(&self.observer, selection, zoomed) {
                        Some(command) => {
                            return Turn::First(vec![Item::Action(Action::Server(command))]);
                        }
                        None => Err(err!(Validation, "no pane selected")),
                    }
                }
                ClientAction::Tab(ClientTab::Move { direction }) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(tab) =
                            selection.and_then(|node| tree::viewed_tab(&replica.tabs, node))
                        && let Some(to) = shift(&replica.tabs, tab, direction)
                    {
                        return Turn::First(vec![Item::Move(TabMove { tab, to })]);
                    }
                    Ok(())
                }
                ClientAction::Tab(ClientTab::Select { .. }) => unavailable("tab select"),
                ClientAction::Tab(ClientTab::Collapse {}) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(tab) =
                            selection.and_then(|node| tree::viewed_tab(&replica.tabs, node))
                    {
                        match collapse(&replica.tabs, &self.sidebar, tab) {
                            Some(Hop::Fold { collapsed }) => self.sidebar.tabs.fold(tab, collapsed),
                            Some(Hop::Select(to)) => {
                                if let Ok(tab) = tree::tab(&replica.tabs, to) {
                                    return self.select(client, self.memory.landing(tab));
                                }
                            }
                            None => {}
                        }
                    }
                    Ok(())
                }
                ClientAction::Tab(ClientTab::Expand {}) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(tab) =
                            selection.and_then(|node| tree::viewed_tab(&replica.tabs, node))
                    {
                        match expand(&replica.tabs, &self.sidebar, tab) {
                            Some(Hop::Fold { collapsed }) => self.sidebar.tabs.fold(tab, collapsed),
                            Some(Hop::Select(to)) => {
                                if let Ok(tab) = tree::tab(&replica.tabs, to) {
                                    return self.select(client, self.memory.landing(tab));
                                }
                            }
                            None => {}
                        }
                    }
                    Ok(())
                }
                ClientAction::Send(chord) => match typing_pane(&self.observer, &self.memory) {
                    Some(pane) => {
                        self.send(InputFrame::Key(KeyInput {
                            pane,
                            key: chord.into(),
                        }));
                        Ok(())
                    }
                    None => Err(err!(Validation, "no pane selected")),
                },
                ClientAction::Agents(ClientAgents::Next {}) => {
                    if let Some(replica) = &self.observer.replica
                        && let Some(pane) =
                            agents::next(&agents::list(&replica.tabs, &self.attention), selection)
                        && Some(NodeId::Pane(pane)) != selection
                    {
                        return self.select(client, NodeId::Pane(pane));
                    }
                    Ok(())
                }
                ClientAction::Run(Run { command }) => {
                    run::start(&self.runner, client, &self.observer, command)
                }
                ClientAction::Detach {} => return Turn::Detach,
                action @ (ClientAction::Mode(_)
                | ClientAction::Sidebar(_)
                | ClientAction::Config(_)) => return Turn::Local(action),
                _ => unreachable!("navigation handled before the action match"),
            },
        };
        if let Err(error) = done {
            self.message = Some(error.to_string());
        }
        Turn::Done
    }
}

/// Tab typing uses its landing pane without changing the tab selection.
fn typing_pane(observer: &Observer, memory: &Memory) -> Option<IdOf<Pane>> {
    let selected = observer.selected()?;
    match selected.pane {
        Some(pane) => Some(pane.id),
        None => match memory.landing(selected.tab) {
            NodeId::Pane(pane) => Some(pane),
            NodeId::Tab(_) => None,
        },
    }
}

/// Clears unseen marks for what this frame shows before drawing it.
#[traced]
fn redraw(guard: &mut TerminalGuard, local: &mut Local) -> Result<()> {
    local.attention.viewed(&local.observer);
    let mut cursor = None;
    guard
        .terminal
        .draw(|frame| cursor = draw::draw(frame, local))
        .map_err(|error| err!(Io, "cannot draw", @external: error))?;
    guard.set_cursor(cursor)
}

fn status<'a>(keys: &'a Keys, message: Option<&'a str>) -> Status<'a> {
    Status {
        mode: keys.shown_mode(),
        message,
        hints: keys
            .shown_mode()
            .and_then(|mode| keys.keymap.modes.get(mode))
            .map(|mode| mode.hints().collect())
            .unwrap_or_default(),
    }
}

fn unavailable(action: &str) -> Result<()> {
    Err(err!(Unavailable, "{} is not available yet", action))
}

/// Actions that move off the selected pane, so a zoomed tab unzooms first.
fn leaves_zoom(action: &Action) -> bool {
    matches!(
        action,
        Action::Client(ClientAction::Pane(
            ClientPane::Next {}
                | ClientPane::Prev {}
                | ClientPane::Focus { .. }
                | ClientPane::Swap { .. }
        ))
    )
}

/// The tab edit for `client.pane.zoom`, against whether the selected pane's
/// tab is zoomed on it. `None` when no pane is selected.
fn zoom(observer: &Observer, selection: Option<NodeId>, zoomed: Switch) -> Option<Command> {
    let Some(NodeId::Pane(pane)) = selection else {
        return None;
    };
    let current = zoomed_pane(observer, selection).is_some();
    Some(set_zoom(pane, zoomed.apply(current).then_some(pane)))
}

/// Zoom `pane`'s tab onto `zoomed`, or unzoom it with `None`.
fn set_zoom(pane: IdOf<Pane>, zoomed: Option<IdOf<Pane>>) -> Command {
    Command::Tab(TabCommand::Set(tab::Set {
        tab: TabTarget {
            id: None,
            pane: Some(pane),
        },
        name: None,
        zoomed: Some(zoomed),
    }))
}

/// The selected pane, when its tab is zoomed onto it.
fn zoomed_pane(observer: &Observer, selection: Option<NodeId>) -> Option<IdOf<Pane>> {
    let Some(NodeId::Pane(pane)) = selection else {
        return None;
    };
    let tabs = &observer.replica.as_ref()?.tabs;
    let tab = tree::tab(tabs, tree::pane_owner(tabs, pane).ok()?).ok()?;
    (tab.zoomed == Some(pane)).then_some(pane)
}

/// Fold an open row with children, else select its parent.
/// Nothing on a top-level tab that's folded or has no children.
fn collapse(tabs: &Tabs, sidebar: &Sidebar, tab: IdOf<Tab>) -> Option<Hop> {
    let row = tree::tab(tabs, tab).ok()?;
    if !row.tabs.is_empty() && !sidebar.tabs.folded(tab) {
        return Some(Hop::Fold { collapsed: true });
    }
    let path = tree::path(tabs, NodeId::Tab(tab))?;
    let NodeId::Tab(parent) = *path.iter().rev().nth(1)? else {
        return None;
    };
    Some(Hop::Select(parent))
}

/// Unfold a folded row, else select its first child. Nothing on a leaf.
fn expand(tabs: &Tabs, sidebar: &Sidebar, tab: IdOf<Tab>) -> Option<Hop> {
    let row = tree::tab(tabs, tab).ok()?;
    let (child, _) = row.tabs.first()?;
    if sidebar.tabs.folded(tab) {
        Some(Hop::Fold { collapsed: false })
    } else {
        Some(Hop::Select(*child))
    }
}

/// One step among siblings or levels, counting folded siblings too.
/// Nothing at an edge.
fn shift(tabs: &Tabs, tab: IdOf<Tab>, direction: TabDirection) -> Option<MoveTab> {
    let siblings = tree::siblings(tabs, tab)?;
    let index = siblings.get_index_of(&tab)?;
    Some(match direction {
        TabDirection::Up => MoveTab::Before(*siblings.get_index(index.checked_sub(1)?)?.0),
        TabDirection::Down => MoveTab::After(*siblings.get_index(index.checked_add(1)?)?.0),
        TabDirection::In => MoveTab::Parent(Some(*siblings.get_index(index.checked_sub(1)?)?.0)),
        TabDirection::Out => {
            let path = tree::path(tabs, NodeId::Tab(tab))?;
            let NodeId::Tab(parent) = *path.iter().rev().nth(1)? else {
                return None;
            };
            MoveTab::After(parent)
        }
    })
}

/// Where `step` moves the selection `from`. Visible tab rows cycle in order and
/// land where `memory` says; from nothing selected they go to the first or
/// last. Panes cycle within their tab; from a tab, to its first or last pane.
/// Focus goes to the neighbor in the tab's published geometry, ties to the
/// panes `memory` selected most recently. `None` when there is nowhere to go.
fn navigate(
    tabs: &Tabs,
    memory: &Memory,
    sidebar: &Sidebar,
    from: Option<NodeId>,
    step: Step,
) -> Option<NodeId> {
    match step {
        Step::Tab { forward } => {
            let rows = sidebar.tabs.visible(tabs);
            let index = match from {
                Some(node) => {
                    let tab = tree::viewed_tab(tabs, node)?;
                    self::step(rows.iter().position(|id| *id == tab)?, rows.len(), forward)
                }
                None if forward => 0,
                None => rows.len().checked_sub(1)?,
            };
            let tab = tree::tab(tabs, *rows.get(index)?).ok()?;
            Some(memory.landing(tab))
        }
        Step::Pane { forward } => {
            let from = from?;
            let tab = tree::tab(tabs, tree::viewed_tab(tabs, from)?).ok()?;
            let panes: Vec<_> = tab
                .layout
                .iter()
                .flat_map(Layout::slots)
                .map(|slot| slot.front().id)
                .collect();
            let index = match from {
                NodeId::Pane(pane) => self::step(
                    panes.iter().position(|id| *id == pane)?,
                    panes.len(),
                    forward,
                ),
                NodeId::Tab(_) if forward => 0,
                NodeId::Tab(_) => panes.len().checked_sub(1)?,
            };
            let pane = *panes.get(index)?;
            Some(NodeId::Pane(pane))
        }
        Step::Focus(direction) => {
            let Some(NodeId::Pane(from)) = from else {
                return None;
            };
            let tab = tree::tab(tabs, tree::pane_owner(tabs, from).ok()?).ok()?;
            let to = tab
                .geometry
                .as_ref()?
                .neighbor(from, direction, memory.recent())?;
            Some(NodeId::Pane(to))
        }
    }
}

/// The index after or before `index` among `len`, wrapping around.
fn step(index: usize, len: usize, forward: bool) -> usize {
    (index + if forward { 1 } else { len - 1 }) % len
}

/// Attach after `delay`; zero for the first attach.
async fn attach_after(client: &Client, request: AttachRequest, delay: Duration) -> Result<Events> {
    time::sleep(delay).await;
    Ok(Box::pin(client.attach(&request).await?))
}

// `select!` evaluates a branch's expression even when its `if` is false, so
// `events.as_mut().unwrap().next(), if events.is_some()` would panic. These
// two turn "nothing there" into "never ready" instead.

/// The next event, or never while disconnected.
async fn next<S: Stream + Unpin>(events: &mut Option<S>) -> Option<S::Item> {
    match events {
        Some(events) => events.next().await,
        None => pending().await,
    }
}

/// At `due`, or never while nothing is due.
async fn until(due: Option<Instant>) {
    match due {
        Some(due) => time::sleep_until(due.into()).await,
        None => pending().await,
    }
}

/// The pending request's result, or never while none is pending.
async fn finish<F: Future + Unpin>(request: &mut Option<F>) -> F::Output {
    match request {
        Some(request) => request.await,
        None => pending().await,
    }
}

/// The server refused a number: this connection's sequence is broken.
fn refused(error: &AppError) -> bool {
    matches!(error.code(), ErrorCode::OutOfOrder)
}

fn terminal_size() -> Result<(u16, u16)> {
    size().map_err(|error| err!(Io, "cannot read the terminal size", @external: error))
}

/// The tab area a `terminal` of (cols, rows) leaves beside `sidebar`.
fn view_area((cols, rows): (u16, u16), sidebar: &Sidebar) -> Size {
    draw::tab_area(cols, rows, sidebar.width())
}

/// Send only the area, never a selection that could overwrite a script's edit.
fn unsent(observer: &Observer, area: Size) -> Option<(IdOf<Attachment>, ViewInput)> {
    (observer.record()?.area != Some(area)).then_some((observer.attachment?, ViewInput { area }))
}

/// Resolves on SIGTERM, SIGHUP or SIGINT, so the terminal is restored before
/// exit. Raw mode turns `C-c` into a key, so SIGINT only comes from `kill`.
#[cfg(unix)]
fn signaled() -> Result<impl Future<Output = ()>> {
    use tokio::signal::unix::{SignalKind, signal};

    let register = |kind| {
        signal(kind).map_err(|error| err!(Io, "cannot register a signal handler", @external: error))
    };
    let mut terminate = register(SignalKind::terminate())?;
    let mut hangup = register(SignalKind::hangup())?;
    let mut interrupt = register(SignalKind::interrupt())?;
    Ok(async move {
        tokio::select! {
            _ = terminate.recv() => {}
            _ = hangup.recv() => {}
            _ = interrupt.recv() => {}
        }
    })
}

#[cfg(not(unix))]
fn signaled() -> Result<impl Future<Output = ()>> {
    Ok(pending())
}

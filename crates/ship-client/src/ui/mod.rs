//! The full-screen client, opened by bare `ship`.

mod agents;
mod attention;
mod connection;
mod draw;
mod keys;
mod lane;
mod memory;
mod menu;
mod mouse;
mod nav;
mod observer;
mod outbox;
mod panes;
mod rename;
mod run;
mod sidebar;
mod terminal;
mod updates;

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
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ship_core::{
    command::{
        Command, Direction, PaneTarget, Run,
        pane::{self, PaneCommand},
    },
    geometry::TabGeometry,
    id::IdOf,
    layout::{Axis, CycleDirection},
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::{
        AttachRequest, ClipboardText, InputFrame, KeyInput, MoveTab, MoveTabBody, PasteInput,
        ResizePane, SelectView, SseEvent,
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
    connection::{Connection, Link, Reattach, refused},
    draw::Status,
    keys::Keys,
    lane::{Item, Lane, MenuAt, Turn},
    memory::Memory,
    menu::{Menu, Picked},
    mouse::{Mouse, Routed},
    nav::{Hop, Step, View, leaves_zoom, set_zoom},
    observer::Observer,
    outbox::Outbox,
    rename::Rename,
    sidebar::{Sidebar, movable},
    terminal::TerminalGuard,
    updates::{Finished, Panel, Tracker},
};
use crate::{
    Client, KeyScope, Scope,
    keymap::{
        Action, AgentSettings, ClientAction, ClientAgents, ClientPane, ClientSettings, ClientTab,
        ClientUpdates, ConfigAction, Menus, ModeName, Sidebar as SidebarAction,
    },
    sound::Speaker,
    updates::{Freshness, UpdateContext},
    watch,
};

/// At most one view PUT per frame.
const FRAME: Duration = Duration::from_millis(16);

type Events = Pin<Box<dyn Stream<Item = Result<SseEvent>>>>;
/// One item per save of the config file.
type Saves = Pin<Box<dyn Stream<Item = ()>>>;
/// A request the loop awaits alongside everything else.
type Request<'a, T = ()> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

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
    context: UpdateContext,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<()> {
    let signaled = signaled()?;
    let (settings, message) = match ClientSettings::load(config) {
        Ok(settings) => (settings, None),
        Err(error) => {
            tracing::warn!(config = %config.display(), %error, "config unusable; using defaults");
            (ClientSettings::defaults(), Some(error.to_string()))
        }
    };
    let (cols, rows) = terminal_size()?;
    let area = draw::tab_area(cols, rows, settings.sidebar_width);
    let selection = match initial_selection {
        InitialSelection::FirstPane => View {
            tabs: &client.tabs().await?,
            selection: None,
            memory: &Memory::default(),
            sidebar: &Sidebar::new(settings.sidebar_width),
        }
        .navigate(Step::Tab(CycleDirection::Next)),
        InitialSelection::None => None,
    };
    let request = AttachRequest {
        selection,
        area: Some(area),
        feed: Default::default(),
    };
    // Every client line names its connection. Each attach swaps in a new
    // span, which every poll enters, as `.instrument` does with a fixed one.
    let span = RefCell::new(tracing::info_span!("client"));
    // Fail on an unreachable server before taking the terminal.
    let (connection, server) =
        tokio::try_join!(Connection::open(client, request, &span), client.health())?;
    let updates = Tracker::new(context, settings.updates.clone(), server)?;
    let exit = {
        let startup = Startup {
            client,
            config,
            settings,
            message,
            updates,
        };
        let mut drive = pin!(drive(startup, connection, signaled, run_output));
        poll_fn(|cx| {
            let span = span.borrow().clone();
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

/// What `drive` starts from, besides the first connection.
struct Startup<'a> {
    client: &'a Client,
    config: &'a Path,
    /// The file's settings, or the defaults when it was unusable.
    settings: ClientSettings,
    /// Why the file was unusable, for the status line.
    message: Option<String>,
    updates: Tracker<'a>,
}

/// The loop: wait for whatever comes next, apply it, run the lane's
/// turns, and redraw when anything visible changed.
async fn drive(
    Startup {
        client,
        config,
        settings,
        message,
        updates,
    }: Startup<'_>,
    mut connection: Connection<'_>,
    signaled: impl Future<Output = ()>,
    run_output: impl Fn() -> Result<File> + Send + Sync + 'static,
) -> Result<Exit> {
    let guard = &mut TerminalGuard::enter()?;
    tokio::pin!(signaled);
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
        note: None,
        updates,
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
    let mut tick = time::interval(FRAME);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    redraw(guard, &mut local)?;
    loop {
        let screen = Rect::new(0, 0, terminal.0, terminal.1);
        let area = view_area(terminal, &local.sidebar);
        // Computed here because a `select!` precondition can't hand its
        // value to the branch.
        let resize = (connection.idle()
            && local.observer.connected
            && local.observer.caught_up(client.written()))
        .then(|| local.observer.resized(area))
        .flatten();
        let effect = tokio::select! {
            link = connection.next(area) => take(link, &mut local, guard)?,
            result = local.outbox.next() => match result {
                Err(error) if refused(&error) => {
                    tracing::warn!(%error, "input refused; reattaching");
                    Some(Effect::Cut)
                }
                Err(error) => {
                    local.message = Some(error.to_string());
                    Some(Effect::Redraw)
                }
                Ok(()) => None,
            },
            // A request's effect redraws when the server publishes it.
            result = lane.finished() => match result {
                Err(error) if refused(&error) => {
                    tracing::warn!(%error, "request refused; reattaching");
                    Some(Effect::Cut)
                }
                Err(error) => {
                    local.message = Some(error.to_string());
                    Some(Effect::Redraw)
                }
                Ok(()) => None,
            },
            Some(ended) = local.ended.recv() => {
                local.message = Some(run::describe(&ended));
                Some(Effect::Redraw)
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
                Some(Effect::Redraw)
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
                None
            }
            () = until(local.updates.due()) => {
                local.updates.check(Freshness::Cached);
                None
            }
            finished = local.updates.finished() => {
                match finished {
                    Finished::Check(result) => {
                        local.note = local.updates.checked(result).or(local.note.take());
                    }
                    Finished::Run(result) => {
                        local.message = local.updates.ran(result).or(local.message.take());
                    }
                }
                Some(Effect::Redraw)
            }
            event = terminal_events.next() => match event {
                Some(Ok(Event::FocusGained | Event::FocusLost)) => {
                    local.attention.set_focused(matches!(event, Some(Ok(Event::FocusGained))));
                    Some(Effect::Redraw)
                }
                Some(Ok(Event::Resize(cols, rows))) => {
                    terminal = (cols, rows);
                    guard.resize(terminal)?;
                    if matches!(local.overlay, Some(Overlay::Menu(_))) {
                        local.overlay = None;
                    }
                    Some(Effect::Redraw)
                }
                Some(Ok(Event::Mouse(event))) if local.overlay.is_none() && guard.mouse() && local.observer.connected => {
                    let mut mouse = mem::take(&mut local.mouse);
                    let was_dragging = mouse.dragging();
                    let routed = mouse.route(event, &local.scene(screen), lane.busy());
                    local.mouse = mouse;
                    let mut redraw = was_dragging || local.mouse.dragging();
                    match routed {
                        Some(Routed::Lane(item)) => lane.push(item),
                        Some(Routed::Resize(edges)) => {
                            for edge in edges {
                                lane.push(Item::Resize(edge));
                            }
                        }
                        Some(Routed::Sidebar(edit)) => {
                            local.sidebar.edit(edit, terminal.0);
                            redraw = true;
                        }
                        None => {}
                    }
                    redraw.then_some(Effect::Redraw)
                }
                Some(Ok(Event::Mouse(_))) if local.overlay.is_none() => None,
                Some(Ok(event)) => {
                    if local.observer.connected {
                        lane.push(Item::Event(event));
                        None
                    } else {
                        if let Some(exit) = local.offline(event, guard, config, terminal.0) {
                            return Ok(exit);
                        }
                        Some(Effect::Redraw)
                    }
                }
                Some(Err(error)) => return Err(err!(Io, "cannot read the terminal", @external: error)),
                None => return Err(err!(Io, "terminal input closed")),
            },
            _ = tick.tick(), if resize.is_some() => {
                if let Some(resize) = resize {
                    connection.put(resize);
                }
                None
            }
            () = &mut signaled => return Ok(Exit::Signaled),
        };
        let mut pass = Pass::default();
        if let Some(exit) = pass.add(effect) {
            return Ok(exit);
        }
        // Events that arrived while the last one was applied or drawn share
        // this pass's redraw instead of each waiting for its own.
        while !pass.cut
            && let Some(link) = connection.ready()
        {
            if let Some(exit) = pass.add(take(link, &mut local, guard)?) {
                return Ok(exit);
            }
        }
        // Keep the screens, but abandon all work from this connection.
        if pass.cut {
            lane.clear();
            let reattach = local
                .cut(view_area(terminal, &local.sidebar))
                .map_or(Reattach::Previous, Reattach::Remembered);
            connection.cut(reattach);
            pass.redraw = true;
        }
        while let Some(item) = lane.next(&local.observer, client.written()) {
            if item.waits_for_input() && !local.outbox.is_empty() {
                lane.first(vec![item]);
                break;
            }
            pass.redraw |= item.redraws();
            let before = local.drawn();
            match local.turn(client, item, screen) {
                Turn::Done => {}
                Turn::Wait(request) => lane.wait(request),
                Turn::First(items) => lane.first(items),
                Turn::Local(action) => {
                    local.message = local
                        .now(action, guard, config, terminal.0)
                        .err()
                        .map(|error| error.to_string());
                    pass.redraw = true;
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
            // A turn that only reaches the server, like a tab switch, redraws
            // on the publication; drawing now would delay the request. An
            // open overlay takes the key itself: typing, moving the menu.
            pass.redraw |= before.overlay.is_some() || local.drawn() != before;
        }
        if pass.redraw {
            redraw(guard, &mut local)?;
        }
    }
}

/// What one event asks of the rest of its pass through the loop. `None`
/// when nothing this client draws changed.
enum Effect {
    Redraw,
    /// Abandon this connection's work and reattach.
    Cut,
    Exit(Exit),
}

/// What the events of one pass asked for, applied after they're all read.
#[derive(Default)]
struct Pass {
    redraw: bool,
    cut: bool,
}

impl Pass {
    /// Record `effect`, handing back an exit for the loop to return.
    fn add(&mut self, effect: Option<Effect>) -> Option<Exit> {
        match effect? {
            Effect::Redraw => self.redraw = true,
            Effect::Cut => self.cut = true,
            Effect::Exit(exit) => return Some(exit),
        }
        None
    }
}

/// At most one overlay captures input before the mode machine.
enum Overlay {
    Menu(Menu),
    Rename(Rename),
    Updates(Panel),
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
    /// An update's one-time note, shown beside `message` until the next
    /// key action.
    note: Option<String>,
    updates: Tracker<'a>,
    attention: Attention,
    speaker: Speaker,
    /// Replaced whole on reload; readers take the piece they need.
    agents: AgentSettings,
    /// Runs outlive this client, including its terminal.
    runner: Runner,
    ended: mpsc::UnboundedReceiver<RunEnded>,
}

/// What a turn can change on this client's screen without the server.
#[derive(PartialEq)]
struct Drawn {
    overlay: Option<mem::Discriminant<Overlay>>,
    mode: Option<ModeName>,
    message: Option<String>,
    note: Option<String>,
    folded: usize,
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

impl Scene<'_> {
    /// `rect`, relative to the pane area, on the terminal.
    fn on_screen(&self, rect: Rect) -> Rect {
        Rect {
            x: self.panes.x.saturating_add(rect.x),
            y: self.panes.y.saturating_add(rect.y),
            ..rect
        }
    }

    /// `at` relative to the pane area, saturating at its edges.
    fn relative(&self, at: Position) -> Position {
        Position::new(
            at.x.saturating_sub(self.panes.x),
            at.y.saturating_sub(self.panes.y),
        )
    }
}

impl<'a> Local<'a> {
    fn drawn(&self) -> Drawn {
        Drawn {
            overlay: self.overlay.as_ref().map(mem::discriminant),
            mode: self.keys.shown_mode().cloned(),
            message: self.message.clone(),
            note: self.note.clone(),
            folded: self.sidebar.tabs.fold_count(),
        }
    }

    /// `None` before the first replica.
    fn view(&self) -> Option<View<'_>> {
        Some(View {
            tabs: self.observer.tabs()?,
            selection: self.observer.selection(),
            memory: &self.memory,
            sidebar: &self.sidebar,
        })
    }

    /// An event from the stream, with its follow-ups: attention, memory,
    /// the sidebar's reveal and `reconcile`. Returns whether anything
    /// visible changed.
    fn apply(&mut self, event: SseEvent) -> bool {
        let screen = matches!(event, SseEvent::Screen(_));
        let attached = matches!(event, SseEvent::Attached(_));
        let previous = self.observer.selection();
        let visible = self.observer.apply(event);
        if attached {
            self.attention.attached(&self.observer);
        } else if !screen {
            self.attention
                .observe(&self.observer, Instant::now(), self.agents.delay);
        }
        if !screen && let Some(replica) = &self.observer.replica {
            let selection = self.observer.selection();
            self.memory.observe(&replica.tabs, selection);
            if previous != selection {
                self.sidebar.tabs.reveal(&replica.tabs, selection);
            }
            return self.reconcile() | visible;
        }
        visible
    }

    fn scene(&self, screen: Rect) -> Scene<'_> {
        let selected = self.observer.selected();
        let tab = selected.as_ref().map(|selected| selected.tab);
        Scene {
            panes: draw::split(screen, self.sidebar.width()).1,
            tabs: self.observer.tabs(),
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

    /// Interpret input once: the overlay's pick, else the mode's item.
    fn read(&mut self, event: Event) -> Option<Item> {
        let item = match &mut self.overlay {
            Some(overlay) => {
                let picked = match overlay {
                    Overlay::Menu(menu) => menu.handle(&event),
                    Overlay::Rename(rename) => rename.handle(&event),
                    Overlay::Updates(panel) => panel.handle(&event),
                };
                match picked {
                    Picked::Run(action) => {
                        self.overlay = None;
                        Some(Item::Action(action))
                    }
                    Picked::Stay(action) => Some(Item::Action(action)),
                    Picked::Close => {
                        self.overlay = None;
                        None
                    }
                    Picked::Open => None,
                }
            }
            None => self.keys.handle(event),
        };
        if matches!(item, Some(Item::Action(_))) {
            self.message = None;
            self.note = None;
        }
        item
    }

    /// Bring client-local state in line with the replica: forget folds and
    /// drop feedback for tabs that changed, and close a rename field whose
    /// target disappeared or is no longer selected by this client. Returns
    /// whether the rename field closed.
    fn reconcile(&mut self) -> bool {
        let Some(replica) = &self.observer.replica else {
            return false;
        };
        self.sidebar.tabs.reconcile(&replica.tabs);
        self.mouse.reconcile(&replica.tabs);
        self.updates.reconcile(&replica.tabs);
        let Some(Overlay::Rename(rename)) = &self.overlay else {
            return false;
        };
        let closed = match rename.target() {
            NodeId::Tab(tab) => tree::tab(&replica.tabs, tab).err().map(|_| "tab closed"),
            NodeId::Pane(pane) => tree::pane(&replica.tabs, pane).err().map(|_| "pane closed"),
        };
        if let Some(message) = closed {
            self.overlay = None;
            self.message = Some(message.into());
            return true;
        }
        let selected = self.observer.selected();
        let moved = match rename.target() {
            NodeId::Tab(tab) => selected.map(|selected| selected.tab.id) != Some(tab),
            NodeId::Pane(pane) => {
                selected
                    .and_then(|selected| selected.pane)
                    .map(|pane| pane.id)
                    != Some(pane)
            }
        };
        if moved {
            self.overlay = None;
        }
        moved
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
            Some(Item::Action(Action::Client(ClientAction::Detach {}))) => {
                return Some(Exit::Detached);
            }
            Some(Item::Action(Action::Client(
                action @ (ClientAction::Mode(_)
                | ClientAction::Sidebar(_)
                | ClientAction::Config(_)),
            ))) => {
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
                self.sidebar.nudge(direction, amount.unwrap_or(1), cols);
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
        self.updates.configure(settings.updates);
        Ok(())
    }

    #[traced]
    fn turn(&mut self, client: &'a Client, item: Item, screen: Rect) -> Turn<'a> {
        match item {
            Item::Event(event) => self
                .read(event)
                .map_or(Turn::Done, |item| Turn::First(vec![item])),
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
                if let Some(pane) = self.view().and_then(|view| view.typing_pane()) {
                    self.send(InputFrame::Key(KeyInput { pane, key }));
                }
                Turn::Done
            }
            Item::Paste(text) => {
                if let Some(pane) = self.view().and_then(|view| view.typing_pane()) {
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
                    .tabs()
                    .is_some_and(|tabs| tree::tab(tabs, tab).is_ok())
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
                    .tabs()
                    .is_some_and(|tabs| movable(tabs, moving.tab, moving.to))
                {
                    return Turn::Done;
                }
                if let MoveTab::Parent(Some(parent)) = moving.to {
                    self.sidebar.tabs.expand(parent);
                }
                let body = MoveTabBody { at: moving.to };
                tolerant(
                    async move { client.move_tab(moving.tab, &body).await },
                    &[ErrorCode::NotFound, ErrorCode::InvalidStructure],
                )
            }
            Item::Resize(position) => {
                if !self
                    .observer
                    .tabs()
                    .is_some_and(|tabs| tree::pane_owner(tabs, position.edge.pane).is_ok())
                {
                    return Turn::Done;
                }
                let body = ResizePane {
                    direction: match position.edge.axis {
                        Axis::Horizontal => Direction::Right,
                        Axis::Vertical => Direction::Down,
                    },
                    amount: None,
                    to: Some(position.to),
                };
                tolerant(
                    async move { client.resize(position.edge.pane, &body).await },
                    &[ErrorCode::NotFound],
                )
            }
        }
    }

    /// Select a clicked target, using the tab's remembered landing pane.
    fn land(&self, client: &'a Client, node: NodeId) -> Turn<'a> {
        match self.view().and_then(|view| view.land(node)) {
            Some(node) => self.select(client, node),
            None => Turn::Done,
        }
    }

    /// Fold the row or select the landing node, for `client.tab.collapse`
    /// and `client.tab.expand`.
    fn hop(&mut self, client: &'a Client, hop: Hop) -> Turn<'a> {
        match hop {
            Hop::Collapse(tab) => self.sidebar.tabs.collapse(tab),
            Hop::Expand(tab) => self.sidebar.tabs.expand(tab),
            Hop::Select(node) => return self.select(client, node),
        }
        Turn::Done
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
        if matches!(self.overlay, Some(Overlay::Menu(_))) {
            self.overlay = None;
        }
        self.observer.connected = false;
        self.observer.remembered(area)
    }

    fn select(&self, client: &'a Client, node: NodeId) -> Turn<'a> {
        let Some(attachment) = self.observer.attachment else {
            return Turn::Done;
        };
        let body = SelectView {
            selection: Some(node),
        };
        tolerant(
            async move { client.select(attachment, &body).await },
            &[ErrorCode::NotFound],
        )
    }

    fn act(&mut self, client: &'a Client, action: Action) -> Turn<'a> {
        // Turns run only while attached, which brings a replica.
        let Some(view) = self.view() else {
            return Turn::Done;
        };
        let selection = view.selection;
        if leaves_zoom(&action)
            && let Some(zoomed) = view.zoomed_pane()
        {
            return Turn::First(vec![
                Item::Action(Action::Server(set_zoom(zoomed, None))),
                Item::Action(action),
            ]);
        }
        if let Some(step) = Step::of(&action) {
            return match view.navigate(step) {
                Some(to) if Some(to) != selection => self.select(client, to),
                _ => Turn::Done,
            };
        }
        let action = match action {
            Action::Server(command) => {
                let scope = Scope::Keys(KeyScope {
                    attachment: self.observer.attachment,
                    selection,
                    tabs: view.tabs.clone(),
                });
                return Turn::Wait(Box::pin(async move {
                    client.execute(command, &scope).await.map(|_| ())
                }));
            }
            Action::Client(action) => action,
        };
        let done = match action {
            ClientAction::Pane(ClientPane::Swap { direction }) => {
                if let Some(NodeId::Pane(from)) = selection
                    && let Some(NodeId::Pane(other)) = view.navigate(Step::Focus(direction))
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
            ClientAction::Pane(ClientPane::Zoom { zoomed }) => match view.zoom(zoomed) {
                Some(command) => return Turn::First(vec![Item::Action(Action::Server(command))]),
                None => Err(err!(Validation, "no pane selected")),
            },
            ClientAction::Tab(ClientTab::Move { direction }) => {
                if let Some(moving) = view.shift(direction) {
                    return Turn::First(vec![Item::Move(moving)]);
                }
                Ok(())
            }
            ClientAction::Tab(ClientTab::Collapse {}) => {
                if let Some(hop) = view.collapse() {
                    return self.hop(client, hop);
                }
                Ok(())
            }
            ClientAction::Tab(ClientTab::Expand {}) => {
                if let Some(hop) = view.expand() {
                    return self.hop(client, hop);
                }
                Ok(())
            }
            ClientAction::Agents(ClientAgents::Next {}) => {
                if let Some(pane) =
                    agents::next(&agents::list(view.tabs, &self.attention), selection)
                    && Some(NodeId::Pane(pane)) != selection
                {
                    return self.select(client, NodeId::Pane(pane));
                }
                Ok(())
            }
            ClientAction::Send(chord) => match view.typing_pane() {
                Some(pane) => {
                    self.send(InputFrame::Key(KeyInput {
                        pane,
                        key: chord.into(),
                    }));
                    Ok(())
                }
                None => Err(err!(Validation, "no pane selected")),
            },
            ClientAction::Tab(ClientTab::Rename {}) => {
                if let Some(selected) = view.selected() {
                    self.overlay = Some(Overlay::Rename(Rename::open(
                        NodeId::Tab(selected.tab.id),
                        selected.tab.name.as_deref(),
                    )));
                }
                Ok(())
            }
            ClientAction::Pane(ClientPane::Rename {}) => {
                if let Some(pane) = view.selected().and_then(|selected| selected.pane) {
                    self.overlay = Some(Overlay::Rename(Rename::open(
                        NodeId::Pane(pane.id),
                        pane.name.as_deref(),
                    )));
                }
                Ok(())
            }
            ClientAction::Tab(ClientTab::Select { .. }) => unavailable("tab select"),
            ClientAction::Run(Run { command }) => {
                run::start(&self.runner, client, &self.observer, command)
            }
            ClientAction::Updates(ClientUpdates::Open {}) => {
                self.overlay = Some(Overlay::Updates(Panel::default()));
                Ok(())
            }
            ClientAction::Updates(ClientUpdates::Check {}) => {
                self.updates.check(Freshness::Force);
                Ok(())
            }
            ClientAction::Updates(ClientUpdates::Update {}) => {
                if let (Some(attachment), Some(replica)) =
                    (self.observer.attachment, &self.observer.replica)
                {
                    self.updates.update(client, attachment, &replica.tabs);
                }
                Ok(())
            }
            ClientAction::Detach {} => return Turn::Detach,
            action @ (ClientAction::Mode(_)
            | ClientAction::Sidebar(_)
            | ClientAction::Config(_)) => {
                return Turn::Local(action);
            }
            _ => unreachable!("navigation handled before the action match"),
        };
        match done {
            Ok(()) => Turn::Done,
            Err(error) => self.fail(error),
        }
    }

    /// Show `error` on the status line.
    fn fail(&mut self, error: AppError) -> Turn<'a> {
        self.message = Some(error.to_string());
        Turn::Done
    }
}

/// Run `request` as a turn, counting the given codes as done: the target
/// vanished before the turn.
fn tolerant<'a, T>(
    request: impl Future<Output = Result<T>> + 'a,
    gone: &'static [ErrorCode],
) -> Turn<'a> {
    Turn::Wait(Box::pin(async move {
        match request.await {
            Err(error) if gone.contains(&error.code()) => Ok(()),
            result => result.map(|_| ()),
        }
    }))
}

/// Apply one message from the connection.
fn take(link: Link, local: &mut Local, guard: &mut TerminalGuard) -> Result<Option<Effect>> {
    Ok(match link {
        Link::Shutdown => Some(Effect::Exit(Exit::ServerStopped)),
        Link::Event(SseEvent::Clipboard(ClipboardText { text })) => {
            guard.copy(&text)?;
            None
        }
        Link::Event(event) => local.apply(event).then_some(Effect::Redraw),
        Link::Lost => Some(Effect::Cut),
        Link::PutFailed(error) => {
            local.message = Some(error.to_string());
            None
        }
    })
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

fn status<'a>(keys: &'a Keys, message: Option<&'a str>, note: Option<&'a str>) -> Status<'a> {
    Status {
        mode: keys.shown_mode(),
        message,
        note,
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

// `select!` evaluates a branch's expression even when its `if` is false, so
// `events.as_mut().unwrap().next(), if events.is_some()` would panic. These
// turn "nothing there" into "never ready" instead.

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

fn terminal_size() -> Result<(u16, u16)> {
    size().map_err(|error| err!(Io, "cannot read the terminal size", @external: error))
}

/// The tab area a `terminal` of (cols, rows) leaves beside `sidebar`.
fn view_area((cols, rows): (u16, u16), sidebar: &Sidebar) -> Size {
    draw::tab_area(cols, rows, sidebar.width())
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

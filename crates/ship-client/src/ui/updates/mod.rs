//! What the client knows about updates for the session: read by the
//! sidebar row, the one-time note and the panel.

mod panel;

pub(super) use panel::Panel;

use std::{
    env,
    ffi::OsString,
    time::{Duration, Instant},
};

use semver::Version;
use ship_core::{
    HealthResponse,
    id::{Attachment, IdOf},
    model::{ExitBehavior, ExitStatus, Pane, PaneStatus, Tab},
    prelude::*,
    protocol::{CreatePane, CreateTab, MoveTab, NewPane, NewTab, PaneAt},
    tree::{self, Tabs},
};

use super::{Request, finish};
use crate::{
    Client,
    updates::{
        Checker, FRESH_FOR, Found, Freshness, Method, Sync, UpdateContext, UpdateSettings,
        installed, probe,
    },
};

/// Clients that reused one result go stale together; a random share of
/// this spreads their next checks so the first to ask refreshes it for all.
const SPREAD: Duration = Duration::from_secs(60);

/// The update's tab and pane once created; the pane's status is the
/// replica's.
#[derive(Clone, Copy)]
pub(super) struct Started {
    tab: IdOf<Tab>,
    pane: IdOf<Pane>,
}

/// What a pending update request produced.
pub(super) enum Done {
    Started(Started),
    Probed(Sync),
}

/// The update's part of the panel's status, read from the replica on each
/// draw (FR-011, FR-013).
pub(super) enum Progress<'a> {
    Idle,
    /// Creating the tab, running, or probing what a clean exit installed.
    Updating,
    Installed(&'a Sync),
    /// While its pane is kept, which shows why.
    Failed,
}

pub(super) struct Tracker<'a> {
    checker: Checker,
    context: UpdateContext,
    settings: UpdateSettings,
    /// From `/health` at startup; a restarted server ends this client.
    server: HealthResponse,
    /// Read at startup: once self-replace swaps the file, Linux reports
    /// this path with " (deleted)".
    exe: Option<OsString>,
    found: Option<Found>,
    /// Polled by `finished`, like the lane's running request.
    checking: Option<Request<'a, Found>>,
    /// Why the last check failed; the previous `found` still stands.
    check_failed: Option<String>,
    /// When the schedule asks next; None while checks are off.
    due: Option<Instant>,
    /// Creating the tab and pane, or probing after a clean exit.
    pending: Option<Request<'a, Done>>,
    /// The last run's, whose tab the next reuses while it exists.
    started: Option<Started>,
    /// The probe's answer for `started`, the one thing the replica can't
    /// say; it stands after the tab closes.
    sync: Option<Sync>,
}

/// A request of the tracker's that finished, for `checked` or `ran`.
pub(super) enum Finished {
    Check(Result<Found>),
    Run(Result<Done>),
}

/// What the sidebar's bottom row draws (FR-006).
pub(super) enum VersionRow<'a> {
    Current(Version),
    Available {
        installed: Version,
        latest: &'a Version,
    },
}

impl<'a> Tracker<'a> {
    /// Reads the cache, and is due now when checks are on.
    pub fn new(
        context: UpdateContext,
        settings: UpdateSettings,
        server: HealthResponse,
    ) -> Result<Self> {
        let checker = Checker::new(context.cache.clone())?;
        Ok(Self {
            found: checker.cached(),
            due: settings.check.then(Instant::now),
            checker,
            context,
            settings,
            server,
            exe: env::current_exe().ok().map(Into::into),
            checking: None,
            check_failed: None,
            pending: None,
            started: None,
            sync: None,
        })
    }

    pub fn due(&self) -> Option<Instant> {
        self.due
    }

    /// Starts a check unless one is pending; the schedule passes `Cached`,
    /// `client.updates.check` passes `Force`. Moves `due` an hour on, which
    /// a successful check then counts from its result's age.
    pub fn check(&mut self, freshness: Freshness) {
        if self.settings.check {
            self.due = Some(Instant::now() + FRESH_FOR);
        }
        if self.checking.is_some() {
            return;
        }
        let checker = self.checker.clone();
        let latest_command = self.settings.latest_command().map(str::to_owned);
        self.checking = Some(Box::pin(async move {
            checker.check(freshness, latest_command.as_deref()).await
        }));
    }

    /// The next pending check or run to finish; never while none is.
    pub async fn finished(&mut self) -> Finished {
        tokio::select! {
            result = finish(&mut self.checking) => Finished::Check(result),
            result = finish(&mut self.pending) => Finished::Run(result),
        }
    }

    /// Stores a finished check; the one-time note when it announces.
    pub fn checked(&mut self, result: Result<Found>) -> Option<String> {
        self.checking = None;
        let checked = match result.and_then(|found| self.checker.announce(found)) {
            Ok(checked) => checked,
            Err(error) => {
                tracing::warn!(%error, "update check failed");
                self.check_failed = Some(error.to_string());
                return None;
            }
        };
        let latest = checked.found.latest.as_ref().map(ToString::to_string);
        tracing::info!(
            latest = latest.as_deref(),
            announce = checked.announce,
            "update check finished"
        );
        self.check_failed = None;
        // Due once the shared result goes stale, so the next check asks
        // instead of reusing it, and a client that reused another's result
        // keeps that result's hour.
        if self.settings.check {
            let age = checked.found.checked_at.elapsed().unwrap_or_default();
            let spread = SPREAD.mul_f64(fastrand::f64());
            self.due = Some(Instant::now() + FRESH_FOR.saturating_sub(age) + spread);
        }
        let note = checked
            .found
            .latest
            .as_ref()
            .filter(|_| checked.announce)
            .map(|latest| format!("ship v{latest} is available · ≡ updates"));
        self.found = Some(checked.found);
        note
    }

    /// The latest release when it's newer than this client.
    pub fn available(&self) -> Option<&Version> {
        self.found
            .as_ref()
            .and_then(|found| found.latest.as_ref())
            .filter(|latest| *latest > &installed())
    }

    pub fn row(&self) -> VersionRow<'_> {
        match self.available() {
            Some(latest) => VersionRow::Available {
                installed: installed(),
                latest,
            },
            None => VersionRow::Current(installed()),
        }
    }

    pub fn progress(&self, tabs: &Tabs) -> Progress<'_> {
        if self.pending.is_some() {
            return Progress::Updating;
        }
        if let Some(sync) = &self.sync {
            return Progress::Installed(sync);
        }
        let pane = self
            .started
            .and_then(|started| tree::pane(tabs, started.pane).ok());
        match pane.map(|pane| &pane.status) {
            None => Progress::Idle,
            Some(PaneStatus::Exited(exit)) if !clean(exit) => Progress::Failed,
            // Running, or exited 0 with the probe about to start.
            Some(_) => Progress::Updating,
        }
    }

    /// `client.updates.update`: a pane running `ship update` for
    /// `attachment`, in the last run's tab or a new tab `update`, unless an
    /// update is under way or this install has no method.
    pub fn update(&mut self, client: &'a Client, attachment: IdOf<Attachment>, tabs: &Tabs) {
        if matches!(self.progress(tabs), Progress::Updating) || self.method() == Method::Unavailable
        {
            return;
        }
        self.sync = None;
        let reused = self
            .started
            .map(|started| started.tab)
            .filter(|&tab| tree::tab(tabs, tab).is_ok());
        self.pending = Some(Box::pin(async move {
            let tab = match reused {
                Some(tab) => tab,
                None => {
                    client
                        .create_tab(&CreateTab {
                            at: MoveTab::top(),
                            tab: NewTab {
                                name: Some("update".into()),
                            },
                            starter: None,
                            select: None,
                        })
                        .await?
                        .id
                }
            };
            // Like the pane menu's `edit scrollback`, the server's own binary.
            let pane = client
                .create_pane(&CreatePane {
                    at: PaneAt::Tab(tab),
                    place: Default::default(),
                    pane: NewPane {
                        command: Some(
                            ["sh", "-c", "exec \"$SHIP_BIN\" update"]
                                .map(Into::into)
                                .into(),
                        ),
                        exit_behavior: Some(ExitBehavior::Keep),
                        ..Default::default()
                    },
                    select: Some(attachment),
                })
                .await?;
            Ok(Done::Started(Started { tab, pane: pane.id }))
        }));
    }

    /// Stores what the pending run produced; the error for the status line
    /// when the tab couldn't be created.
    pub fn ran(&mut self, result: Result<Done>) -> Option<String> {
        self.pending = None;
        match result {
            Ok(Done::Started(started)) => {
                self.started = Some(started);
                None
            }
            Ok(Done::Probed(sync)) => {
                self.sync = Some(sync);
                None
            }
            Err(error) => {
                tracing::warn!(%error, "cannot start the update");
                Some(error.to_string())
            }
        }
    }

    /// After each publication: probes the installed ship once the update's
    /// pane exits 0.
    pub fn reconcile(&mut self, tabs: &Tabs) {
        let Some(started) = self.started else {
            return;
        };
        let exited_clean = tree::pane(tabs, started.pane)
            .is_ok_and(|pane| matches!(&pane.status, PaneStatus::Exited(exit) if clean(exit)));
        if !exited_clean || self.sync.is_some() || self.pending.is_some() {
            return;
        }
        // `ship update` probes the same binary: the one it replaced, or
        // `ship` on PATH after a command.
        let program = match self.method() {
            Method::SelfReplace => self.exe.clone(),
            _ => None,
        }
        .unwrap_or_else(|| "ship".into());
        let server = self.server.clone();
        self.pending = Some(Box::pin(async move {
            Ok(Done::Probed(Sync::of(probe(&program).await, &server)))
        }));
    }

    fn method(&self) -> Method {
        self.settings.method(&self.context)
    }

    /// On config reload; turning checks on makes it due now.
    pub fn configure(&mut self, settings: UpdateSettings) {
        self.due = match (self.settings.check, settings.check) {
            (_, false) => None,
            (false, true) => Some(Instant::now()),
            (true, true) => self.due,
        };
        self.settings = settings;
    }
}

fn clean(exit: &ExitStatus) -> bool {
    exit.code == 0 && exit.signal.is_none()
}

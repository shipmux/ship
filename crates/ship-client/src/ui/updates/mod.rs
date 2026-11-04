//! What the client knows about updates for the session: read by the
//! sidebar row, the one-time note and the panel.

use std::time::{Duration, Instant};

use semver::Version;
use ship_core::{HealthResponse, prelude::*};

use super::Request;
use crate::updates::{
    Checker, FRESH_FOR, Found, Freshness, UpdateContext, UpdateSettings, installed,
};

/// Clients that reused one result go stale together; a random share of
/// this spreads their next checks so the first to ask refreshes it for all.
const SPREAD: Duration = Duration::from_secs(60);

pub(super) struct Tracker<'a> {
    checker: Checker,
    #[expect(dead_code, reason = "the panel's update method reads it in slice 5")]
    context: UpdateContext,
    settings: UpdateSettings,
    /// From `/health` at startup; a restarted server ends this client.
    #[expect(dead_code, reason = "the panel's server row reads it in slice 5")]
    server: HealthResponse,
    installed: Version,
    found: Option<Found>,
    /// Polled with `finish`, like the lane's running request.
    checking: Option<Request<'a, Found>>,
    /// Why the last check failed; the previous `found` still stands.
    check_failed: Option<String>,
    /// When the schedule asks next; None while checks are off.
    due: Option<Instant>,
}

/// What the sidebar's bottom row draws (FR-006).
pub(super) enum VersionRow<'a> {
    Current(&'a Version),
    Available {
        installed: &'a Version,
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
            installed: installed(),
            checking: None,
            check_failed: None,
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

    /// The pending check, for `finish` in the loop.
    pub fn pending_check(&mut self) -> &mut Option<Request<'a, Found>> {
        &mut self.checking
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

    pub fn row(&self) -> VersionRow<'_> {
        match self.found.as_ref().and_then(|found| found.latest.as_ref()) {
            Some(latest) if latest > &self.installed => VersionRow::Available {
                installed: &self.installed,
                latest,
            },
            _ => VersionRow::Current(&self.installed),
        }
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

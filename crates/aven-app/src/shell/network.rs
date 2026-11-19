//! Which proxy the person's pages go through.

use aven_core::proxy::{self, ProxyServer};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ProxyPreference {
    mode: String,
    address: String,
}

impl ProxyPreference {
    pub(super) fn load(store: &dyn aven_core::ports::store::Store) -> Self {
        let mut preference = Self::default();
        for key in ["network.proxy", "network.proxy-address"] {
            if let Some(value) = store.app_setting(key) {
                preference.apply(key, &value);
            }
        }
        preference
    }

    /// Takes a validated value; anything else leaves the choice as it was.
    /// Returns whether the proxy in force may have changed.
    pub(super) fn apply(&mut self, key: &str, value: &str) -> bool {
        match key {
            "network.proxy" => self.mode = value.to_owned(),
            "network.proxy-address" => self.address = value.to_owned(),
            _ => return false,
        }
        true
    }

    /// The manual proxy, or none for the system's.
    pub(super) fn server(&self) -> Option<ProxyServer> {
        proxy::from_settings(&self.mode, &self.address)
    }
}

/// How fast the person lets each page use the network, and when.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NetworkLimit {
    megabits: u32,
    always: bool,
}

impl NetworkLimit {
    pub(super) fn load(store: &dyn aven_core::ports::store::Store) -> Self {
        let mut limit = Self::default();
        for key in ["performance.network-cap", "performance.network-cap-when"] {
            if let Some(value) = store.app_setting(key) {
                limit.apply(key, &value);
            }
        }
        limit
    }

    /// Takes a validated value; anything else means no limit. Returns
    /// whether the limit in force may have changed.
    pub(super) fn apply(&mut self, key: &str, value: &str) -> bool {
        match key {
            "performance.network-cap" => self.megabits = value.parse().unwrap_or(0),
            "performance.network-cap-when" => self.always = value == "always",
            _ => return false,
        }
        true
    }

    /// The limit in force, in kilobits per second; 0 for none.
    pub(super) fn kilobits(&self, game_in_front: bool) -> u32 {
        if self.always || game_in_front {
            self.megabits.saturating_mul(1000)
        } else {
            0
        }
    }
}

impl super::Shell {
    /// Tells the engine the network limit in force, when it changed.
    pub(super) fn publish_network_limit(&mut self) {
        let wanted = self.network_limit.kilobits(self.game_in_front);
        if self.network_limit_sent != Some(wanted) {
            self.network_limit_sent = Some(wanted);
            self.engine.set_network_limit(wanted);
        }
    }
}

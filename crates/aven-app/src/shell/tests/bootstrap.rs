use super::*;

#[test]
fn first_run_profile_is_persisted_before_profile_scoped_management_is_exposed() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _engine, _screen) = setup_with(store.clone());

    shell.handle(Command::Bootstrap);

    assert!(shell.bootstrapped);
    let focused = shell.windows.focused().unwrap();
    let saved = store.saved.lock().unwrap().clone().unwrap();
    assert!(saved
        .profiles
        .iter()
        .any(|profile| profile.id == focused.profile));
    assert!(saved
        .spaces
        .iter()
        .any(|space| space.id == focused.space && space.profile == focused.profile));
}

// With nothing chosen, launch picks up where the person left off: the tab
// they had in front is in front again, loaded, and no blank tab is added.
#[test]
fn default_startup_returns_to_the_tab_left_in_front() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let previous = active_id(&screen);
    navigate_and_commit(&mut shell, previous, "example.com");

    let (mut restarted, _engine, screen) = setup_with(store);
    restarted.handle(Command::Bootstrap);
    assert_eq!(active_id(&screen), previous);
    let state = last(&screen);
    assert_eq!(state.tabs.len(), 1);
    assert_eq!(state.tabs[0].url.as_deref(), Some("https://example.com/"));
    assert!(restarted.items.tab(previous).unwrap().has_view());
}

#[test]
fn new_tab_startup_opens_a_new_tab_without_losing_or_loading_restored_pages() {
    let store = Arc::new(FakeStore::default());
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "new-tab".into());
    let (mut shell, _, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let previous = active_id(&screen);
    navigate_and_commit(&mut shell, previous, "example.com");

    let (mut restarted, engine, screen) = setup_with(store.clone());
    restarted.handle(Command::Bootstrap);
    let fresh = active_id(&screen);
    assert_ne!(fresh, previous);
    let state = last(&screen);
    assert_eq!(state.tabs.len(), 2);
    assert!(state
        .tabs
        .iter()
        .any(|tab| tab.id == previous.to_string()
            && tab.url.as_deref() == Some("https://example.com/")));
    assert_eq!(restarted.items.tab(fresh).unwrap().url, None);
    assert!(engine
        .calls()
        .iter()
        .all(|call| !call.contains(&previous.to_string())));

    restarted.handle(Command::Bootstrap);
    assert_eq!(active_id(&screen), fresh);
    assert_eq!(last(&screen).tabs.len(), 2);
    restarted.persist();

    let (mut again, _, screen) = setup_with(store);
    again.handle(Command::Bootstrap);
    assert_eq!(again.items.tab(active_id(&screen)).unwrap().url, None);
    assert_eq!(last(&screen).tabs.len(), 2);
}

#[test]
fn new_tab_startup_restores_split_views_when_a_member_is_selected() {
    let store = Arc::new(FakeStore::default());
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "new-tab".into());
    let (mut shell, _, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "example.com");
    shell.handle(Command::Open);
    let second = active_id(&screen);
    navigate_and_commit(&mut shell, second, "github.com");
    shell.handle(Command::SplitWith {
        other: first,
        axis: Axis::Row,
    });
    shell.persist();

    let (mut restarted, engine, screen) = setup_with(store);
    restarted.handle(Command::Bootstrap);
    assert_ne!(active_id(&screen), second);
    assert!(!restarted.items.tab(first).unwrap().has_view());
    assert!(!restarted.items.tab(second).unwrap().has_view());
    restarted.handle(Command::Activate(second));
    let panes = engine.last_layout();
    assert_eq!(panes.len(), 2);
    assert!(panes.contains(&first.to_string()) && panes.contains(&second.to_string()));
}

#[test]
fn new_tab_startup_at_the_item_limit_keeps_the_restored_active_tab() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let previous = active_id(&screen);
    navigate_and_commit(&mut shell, previous, "example.com");
    let space = shell.windows.focused().unwrap().space;
    for _ in 1..aven_core::session::MAX_SESSION_ITEMS {
        assert!(shell.items.insert_unloaded_tab(
            ItemId::generate(),
            Placement::Space {
                space,
                section: SpaceSection::Today
            },
            url::Url::parse("https://example.com/").unwrap(),
            "Example",
        ));
    }
    shell.persist();

    let (mut restarted, _, screen) = setup_with(store);
    restarted.handle(Command::Bootstrap);
    assert_eq!(active_id(&screen), previous);
    assert_eq!(
        last(&screen).tabs.len(),
        aven_core::session::MAX_SESSION_ITEMS
    );
    assert!(restarted.items.tab(previous).unwrap().has_view());
}

#[test]
fn restart_preserves_ids_actives_and_splits() {
    let store = Arc::new(FakeStore::default());
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "continue".into());
    let (mut shell, _engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "example.com");
    shell.handle(Command::Open);
    let second = active_id(&screen);
    navigate_and_commit(&mut shell, second, "github.com");
    shell.handle(Command::SplitWith {
        other: first,
        axis: Axis::Row,
    });

    let before = last(&screen);

    let (mut shell2, engine2, screen2) = setup_with(store);
    shell2.handle(Command::Bootstrap);
    let after = last(&screen2);

    // same ULIDs, same order, same active tab survive the restart
    let ids = |s: &ItemsState| s.tabs.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&after), ids(&before));
    assert_eq!(after.active, before.active);
    // the split tree is restored and both panes get views again
    let panes = engine2.last_layout();
    assert_eq!(panes.len(), 2);
    assert!(panes.contains(&first.to_string()) && panes.contains(&second.to_string()));
}

#[test]
fn the_engine_learns_the_proxy_before_any_view_and_whenever_it_changes() {
    let store = Arc::new(FakeStore::default());
    store.settings.lock().unwrap().extend([
        ("network.proxy".to_owned(), "http".to_owned()),
        (
            "network.proxy-address".to_owned(),
            "proxy.example.com:3128".to_owned(),
        ),
    ]);
    let (mut shell, engine, _screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    assert_eq!(
        engine.proxies(),
        [Some(
            "--proxy-server=http://proxy.example.com:3128".to_owned()
        )]
    );

    shell.handle(Command::SetAppSetting {
        key: "network.proxy".into(),
        value: "socks5".into(),
    });
    shell.handle(Command::SetAppSetting {
        key: "network.proxy-address".into(),
        value: "".into(),
    });
    assert_eq!(
        engine.proxies(),
        [
            Some("--proxy-server=http://proxy.example.com:3128".to_owned()),
            Some("--proxy-server=socks5://proxy.example.com:3128".to_owned()),
            None,
        ]
    );
}

#[test]
fn the_engine_learns_forced_dark_before_any_view_and_whenever_it_changes() {
    let dark = |engine: &FakeEngine| engine.page_preferences("force-dark");
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, _screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    assert_eq!(
        dark(&engine).first().map(String::as_str),
        Some("force-dark false")
    );

    shell.handle(Command::SetAppSetting {
        key: "ui.force-dark".into(),
        value: "true".into(),
    });
    assert_eq!(
        dark(&engine).last().map(String::as_str),
        Some("force-dark true")
    );
    shell.handle(Command::SetAppSetting {
        key: "ui.force-dark".into(),
        value: "false".into(),
    });
    assert_eq!(
        dark(&engine).last().map(String::as_str),
        Some("force-dark false")
    );

    // Kept across a restart, and told before the first view is made.
    store
        .settings
        .lock()
        .unwrap()
        .insert("ui.force-dark".to_owned(), "true".to_owned());
    let (mut again, engine2, _screen2) = setup_with(store);
    again.handle(Command::Bootstrap);
    assert_eq!(
        dark(&engine2).first().map(String::as_str),
        Some("force-dark true")
    );
}

#[test]
fn the_network_limit_holds_while_a_game_is_in_front_or_always_when_asked() {
    let limits = |engine: &FakeEngine| engine.page_preferences("network-limit");
    let set = |shell: &mut Shell, key: &str, value: &str| {
        shell.handle(Command::SetAppSetting {
            key: key.into(),
            value: value.into(),
        });
    };
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, _screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    assert_eq!(
        limits(&engine),
        ["network-limit 0"],
        "no limit until one is chosen"
    );

    // Chosen for games only: nothing changes until a game is in front.
    set(&mut shell, "performance.network-cap", "5");
    assert_eq!(limits(&engine), ["network-limit 0"]);
    shell.handle(Command::SetGameInFront(true));
    shell.handle(Command::SetGameInFront(true));
    assert_eq!(limits(&engine), ["network-limit 0", "network-limit 5000"]);
    shell.handle(Command::SetGameInFront(false));
    assert_eq!(
        limits(&engine).last().map(String::as_str),
        Some("network-limit 0")
    );

    // Asked for all the time: in force with no game, and a new value at once.
    set(&mut shell, "performance.network-cap-when", "always");
    assert_eq!(
        limits(&engine).last().map(String::as_str),
        Some("network-limit 5000")
    );
    set(&mut shell, "performance.network-cap", "25");
    assert_eq!(
        limits(&engine).last().map(String::as_str),
        Some("network-limit 25000")
    );

    // Kept across a restart, and in force before the first view.
    let (mut again, engine2, _screen2) = setup_with(store);
    again.handle(Command::Bootstrap);
    assert_eq!(limits(&engine2), ["network-limit 25000"]);

    // A value that is not a number means no limit, never a wrong one.
    let mut limit = super::super::network::NetworkLimit::default();
    limit.apply("performance.network-cap", "fast");
    assert_eq!(limit.kilobits(true), 0);
}

#[test]
fn consent_banners_are_refused_unless_the_person_said_not_to() {
    let told = |engine: &FakeEngine| engine.page_preferences("consent-refusal");
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, _screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    assert_eq!(told(&engine), ["consent-refusal true"]);

    // Said twice, told once: the engine registers its script per change.
    for _ in 0..2 {
        shell.handle(Command::SetAppSetting {
            key: "privacy.cookie-banners".into(),
            value: "false".into(),
        });
    }
    assert_eq!(
        told(&engine),
        ["consent-refusal true", "consent-refusal false"]
    );

    let (mut again, engine2, _screen2) = setup_with(store);
    again.handle(Command::Bootstrap);
    assert_eq!(told(&engine2), ["consent-refusal false"]);
}

#[test]
fn a_site_set_apart_keeps_its_own_colours_while_pages_are_darkened() {
    let told = |engine: &FakeEngine| engine.page_preferences("page-dark");
    let set = |shell: &mut Shell, key: &str, value: &str| {
        shell.handle(Command::SetAppSetting {
            key: key.into(),
            value: value.into(),
        });
    };
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "example.com");

    // Nothing is said page by page while no site is set apart.
    set(&mut shell, "ui.force-dark", "true");
    assert!(told(&engine).is_empty());

    set(&mut shell, "ui.force-dark-except", "example.com");
    assert_eq!(told(&engine), [format!("page-dark {tab} false")]);

    // Leaving for another site, the page is darkened again.
    navigate_and_commit(&mut shell, tab, "other.example");
    assert_eq!(told(&engine).last(), Some(&format!("page-dark {tab} true")));

    // What is not a list of sites is refused and changes nothing.
    let before = told(&engine).len();
    set(
        &mut shell,
        "ui.force-dark-except",
        "https://other.example/x",
    );
    assert_eq!(told(&engine).len(), before);
}

#[test]
fn sites_whose_banners_are_left_alone_are_told_to_the_engine() {
    let told = |engine: &FakeEngine| engine.page_preferences("consent-except");
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, _screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    assert_eq!(told(&engine), ["consent-except "]);
    shell.handle(Command::SetAppSetting {
        key: "privacy.cookie-banners-except".into(),
        value: "example.com\nwww.news.example".into(),
    });
    assert_eq!(
        told(&engine).last().map(String::as_str),
        Some("consent-except example.com,news.example")
    );

    // Kept across a restart, and told before the first view.
    let (mut again, engine2, _screen2) = setup_with(store);
    again.handle(Command::Bootstrap);
    assert_eq!(told(&engine2), ["consent-except example.com,news.example"]);
}

#[test]
fn a_tab_volume_reaches_the_engine_only_for_a_tab_that_exists() {
    let told = |engine: &FakeEngine| engine.page_preferences("tab-volume");
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    let tab = active_id(&screen);
    shell.handle(Command::SetTabVolume {
        id: tab,
        percent: 40,
    });
    shell.handle(Command::SetTabVolume {
        id: tab,
        percent: 250,
    });
    shell.handle(Command::SetTabVolume {
        id: ItemId::generate(),
        percent: 10,
    });
    assert_eq!(
        told(&engine),
        [
            format!("tab-volume {tab} 40"),
            format!("tab-volume {tab} 100")
        ]
    );
}

#[test]
fn the_page_in_front_is_named_with_its_address_once_it_has_loaded() {
    let ask = |shell: &mut Shell| {
        let (reply, answer) = std::sync::mpsc::sync_channel(1);
        shell.handle(Command::PageInFront { reply });
        answer.try_recv().expect("an answer")
    };
    let store = Arc::new(FakeStore::default());
    let (mut shell, _engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "example.com");
    let (id, address) = ask(&mut shell).expect("the page in front");
    assert_eq!(id, tab);
    assert!(address.contains("example.com"), "{address}");
}

#[test]
fn a_saved_sign_in_reaches_only_the_page_of_its_own_site() {
    let fills = |engine: &FakeEngine| -> usize {
        engine
            .calls()
            .iter()
            .filter(|call| call.starts_with("fill-login"))
            .count()
    };
    let ask = |shell: &mut Shell, address: &str| -> bool {
        let (reply, answer) = std::sync::mpsc::sync_channel(1);
        shell.handle(Command::FillLogin {
            fill: aven_core::login_fill::LoginFill::new(address, "me".into(), "secret".into())
                .expect("fill"),
            reply,
        });
        answer.try_recv().expect("an answer")
    };
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "example.com");

    // Another site's sign-in changes nothing and is never handed over.
    assert!(!ask(&mut shell, "https://other.example/login"));
    assert_eq!(fills(&engine), 0);

    assert!(ask(&mut shell, "https://www.example.com/account/login"));
    assert_eq!(fills(&engine), 1);
    // What the engine is handed never prints the values.
    assert!(engine.calls().iter().all(|call| !call.contains("secret")));
}

#[test]
fn pages_start_below_the_band_the_chrome_keeps_for_its_window_controls() {
    let region = |engine: &FakeEngine| {
        engine
            .layout_regions
            .lock()
            .unwrap()
            .iter()
            .rev()
            .flatten()
            .next()
            .copied()
    };
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store);
    // Said before any window exists, as the host does: the first layout has it.
    shell.handle(Command::SetCaptionBand(28.0));
    shell.handle(Command::Bootstrap);
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "example.com");
    let below = region(&engine).expect("a page region");
    assert_eq!(below.y, 36.0, "the gutter, then the band");

    shell.handle(Command::SetCaptionBand(0.0));
    let flush = region(&engine).expect("a page region");
    assert_eq!(flush.y, 8.0);
    assert_eq!(
        flush.height,
        below.height + 28.0,
        "the band is taken off the top only"
    );
    assert_eq!((flush.x, flush.width), (below.x, below.width));

    // Nonsense is no band, and a huge one is bounded: the page stays in the window.
    for band in [f64::NAN, f64::INFINITY, -5.0] {
        shell.handle(Command::SetCaptionBand(band));
        assert_eq!(region(&engine).expect("a page region").y, 8.0);
    }
    shell.handle(Command::SetCaptionBand(10_000.0));
    assert_eq!(region(&engine).expect("a page region").y, 8.0 + 64.0);
}

#[test]
fn starting_with_pages_opens_the_list_in_front_and_loads_no_restored_tab() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "example.com");
    store.settings.lock().unwrap().extend([
        ("tabs.startup".to_owned(), "pages".to_owned()),
        (
            "tabs.startup-pages".to_owned(),
            "docs.example.org\nnot a page\nhttps://news.example.net/a".to_owned(),
        ),
    ]);

    let (mut shell2, engine2, screen2) = setup_with(store);
    shell2.handle(Command::Bootstrap);
    let after = last(&screen2);
    assert!(after.tabs.iter().any(|tab| tab.id == first.to_string()));
    assert_eq!(after.tabs.len(), 3);
    let front = active_id(&screen2);
    assert_ne!(front, first);
    let calls = engine2.calls();
    let created: Vec<_> = calls
        .iter()
        .filter(|call| call.starts_with("create "))
        .collect();
    assert_eq!(created.len(), 2, "{calls:?}");
    assert!(created[0].ends_with("https://docs.example.org/ [default]"));
    assert!(created[0].contains(&front.to_string()));
    assert!(created[1].ends_with("https://news.example.net/a [default]"));
    assert!(calls.iter().all(|call| !call.contains(&first.to_string())));
}

#[test]
fn an_empty_list_of_startup_pages_starts_where_the_person_left_off() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "example.com");
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".to_owned(), "pages".to_owned());

    let (mut shell2, _engine2, screen2) = setup_with(store);
    shell2.handle(Command::Bootstrap);
    assert_eq!(active_id(&screen2), first);
}

#[test]
fn starting_with_a_new_tab_keeps_every_tab_and_loads_none_of_them() {
    let store = Arc::new(FakeStore::default());
    let (mut shell, _engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "example.com");
    shell.handle(Command::Open);
    let second = active_id(&screen);
    navigate_and_commit(&mut shell, second, "github.com");
    shell.handle(Command::SplitWith {
        other: first,
        axis: Axis::Row,
    });
    shell.handle(Command::SetAppSetting {
        key: "tabs.startup".into(),
        value: "new-tab".into(),
    });
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "new-tab".into());

    let (mut shell2, engine2, screen2) = setup_with(store);
    shell2.handle(Command::Bootstrap);
    let after = last(&screen2);
    let fresh = active_id(&screen2);
    assert!(![first, second].contains(&fresh));
    assert!(after.tabs.iter().any(|tab| tab.id == first.to_string()));
    assert!(after.tabs.iter().any(|tab| tab.id == second.to_string()));
    assert_eq!(
        after
            .tabs
            .iter()
            .find(|tab| tab.id == fresh.to_string())
            .unwrap()
            .url,
        None
    );
    assert!(engine2
        .calls()
        .iter()
        .all(|call| !call.contains(&first.to_string()) && !call.contains(&second.to_string())));
}

#[test]
fn qa_settings_tab_is_dropped_and_first_bootstrap_opens_a_usable_new_tab() {
    let profile = ProfileId::from(9200);
    let space = SpaceId::from(9201);
    let settings = ItemId::from(9202);
    let store = Arc::new(FakeStore {
        saved: Mutex::new(Some(SessionState {
            profiles: vec![PersistedProfile {
                id: profile,
                name: "Personal".into(),
                kind: ProfileKind::Default,
            }],
            spaces: vec![PersistedSpace {
                id: space,
                profile,
                name: "Today".into(),
                look: Default::default(),
            }],
            items: vec![PersistedItem {
                id: settings,
                parent: None,
                placement: Placement::Space {
                    space,
                    section: SpaceSection::Today,
                },
                kind: PersistedKind::BrowserTab {
                    page: aven_core::item::BrowserOwnedTab::Settings,
                },
            }],
            active_space: Some(space),
            active_item: Some(settings),
            splits: None,
            recently_closed: Vec::new(),
        })),
        ..Default::default()
    });
    let (mut shell, engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);
    assert_ne!(active_id(&screen), settings);
    assert!(shell.items.tab(settings).is_none());
    assert_eq!(shell.active_browser_page(), None);
    assert_eq!(
        shell.browser_page_projected,
        Some((shell.windows.focused().unwrap().id, None)),
    );
    assert!(engine.last_layout().is_empty());
}

#[test]
fn bootstrap_never_creates_views_for_foreign_focus_or_split_references() {
    let local_profile = ProfileId::from(9100);
    let foreign_profile = ProfileId::from(9101);
    let local_space = SpaceId::from(9102);
    let sibling_space = SpaceId::from(9103);
    let foreign_space = SpaceId::from(9104);
    let local = ItemId::from(9105);
    let sibling = ItemId::from(9106);
    let foreign = ItemId::from(9107);
    let tab = |id, space, host: &str| PersistedItem {
        id,
        parent: None,
        placement: Placement::Space {
            space,
            section: SpaceSection::Today,
        },
        kind: PersistedKind::Tab {
            url: format!("https://{host}/"),
            title: host.into(),
            zoom: 1.0,
            last_active_ms: None,
        },
    };
    let store = Arc::new(FakeStore {
        saved: Mutex::new(Some(SessionState {
            profiles: vec![
                PersistedProfile {
                    id: local_profile,
                    name: "Local".into(),
                    kind: ProfileKind::Default,
                },
                PersistedProfile {
                    id: foreign_profile,
                    name: "Foreign".into(),
                    kind: ProfileKind::Named,
                },
            ],
            spaces: vec![
                PersistedSpace {
                    id: local_space,
                    profile: local_profile,
                    name: "Local".into(),
                    look: Default::default(),
                },
                PersistedSpace {
                    id: sibling_space,
                    profile: local_profile,
                    name: "Sibling".into(),
                    look: Default::default(),
                },
                PersistedSpace {
                    id: foreign_space,
                    profile: foreign_profile,
                    name: "Foreign".into(),
                    look: Default::default(),
                },
            ],
            items: vec![
                tab(local, local_space, "local.example"),
                tab(sibling, sibling_space, "sibling.example"),
                tab(foreign, foreign_space, "foreign.example"),
            ],
            active_space: Some(local_space),
            active_item: Some(foreign),
            splits: Some(Pane::Branch {
                axis: Axis::Row,
                ratio: 0.5,
                a: Box::new(Pane::Leaf(local)),
                b: Box::new(Pane::Leaf(sibling)),
            }),
            recently_closed: Vec::new(),
        })),
        ..Default::default()
    });

    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "continue".into());
    let (mut shell, engine, screen) = setup_with(store);
    shell.handle(Command::Bootstrap);

    let win = shell.windows.focused().unwrap();
    assert_eq!(win.profile, local_profile);
    assert_eq!(win.space, local_space);
    assert_eq!(win.active, Some(local));
    assert!(win.splits.is_none());
    assert_eq!(active_id(&screen), local);
    assert_eq!(engine.last_layout(), vec![local.to_string()]);
    let calls = engine.calls();
    assert!(calls
        .iter()
        .any(|call| call.starts_with(&format!("create {local} "))));
    assert!(!calls
        .iter()
        .any(|call| call.starts_with(&format!("create {sibling} "))));
    assert!(!calls
        .iter()
        .any(|call| call.starts_with(&format!("create {foreign} "))));
}

#[test]
fn restart_discards_oversized_split_before_creating_views() {
    let store = Arc::new(FakeStore::default());
    store
        .settings
        .lock()
        .unwrap()
        .insert("tabs.startup".into(), "continue".into());
    let (mut shell, _engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    let mut ids = vec![active_id(&screen)];
    navigate_and_commit(&mut shell, ids[0], "pane0.example");
    for n in 1..=MAX_VISIBLE_PANES {
        shell.handle(Command::Open);
        let id = active_id(&screen);
        navigate_and_commit(&mut shell, id, &format!("pane{n}.example"));
        ids.push(id);
    }

    let mut state = store.saved.lock().unwrap().clone().unwrap();
    state.active_item = Some(ids[0]);
    state.splits = Some(
        ids[1..]
            .iter()
            .fold(Pane::Leaf(ids[0]), |tree, id| Pane::Branch {
                axis: Axis::Row,
                ratio: 0.5,
                a: Box::new(tree),
                b: Box::new(Pane::Leaf(*id)),
            }),
    );
    *store.saved.lock().unwrap() = Some(state);

    let (mut restored, engine, _screen) = setup_with(store);
    restored.handle(Command::Bootstrap);
    assert_eq!(engine.last_layout(), vec![ids[0].to_string()]);
    assert!(restored.windows.focused().unwrap().splits.is_none());
    assert_eq!(
        engine
            .calls()
            .iter()
            .filter(|call| call.starts_with("create "))
            .count(),
        1,
        "oversized restored panes must not trigger a controller storm"
    );
}

#[test]
fn bootstrap_is_idempotent_across_chrome_reloads() {
    let (mut shell, engine, screen) = setup();
    shell.handle(Command::Bootstrap);
    let first = active_id(&screen);
    shell.handle(Command::Navigate {
        id: first,
        input: "example.com".into(),
    });

    shell.handle(Command::SetSidebarHidden(true));
    shell.handle(Command::PeekSidebar {
        peek: true,
        slide: false,
    });
    let window = shell.windows.focused().unwrap().id;
    // chrome webview reloaded (dev HMR): same window, same stage, state kept
    shell.handle(Command::Bootstrap);
    assert_eq!(
        engine
            .calls()
            .into_iter()
            .filter(|call| call.starts_with("raise@"))
            .collect::<Vec<_>>(),
        [
            format!("raise@{window} false"),
            format!("raise@{window} true"),
            format!("raise@{window} false")
        ]
    );
    assert_eq!(active_id(&screen), first);
    let windows: std::collections::HashSet<String> = engine
        .calls()
        .iter()
        .filter_map(|c| c.split(' ').next().map(String::from))
        .filter(|c| c.starts_with("layout@"))
        .collect();
    assert_eq!(windows.len(), 1, "one window, one stage: {windows:?}");
    assert_eq!(last(&screen).tabs.len(), 1);
}

#[test]
fn exhausted_space_aggregate_refuses_first_run_without_panicking() {
    let (mut shell, _engine, _screen) = setup();
    let profile = ProfileId::from(26_200);
    assert!(shell.profiles.insert(Profile {
        id: profile,
        name: "Personal".into(),
        kind: ProfileKind::Default,
    }));
    for index in 0..aven_core::session::MAX_SESSION_SPACES {
        assert!(shell.spaces.insert(Space {
            id: SpaceId::from(27_000 + index as u128),
            profile,
            name: "Full".into(),
            look: Default::default(),
        }));
    }

    assert!(shell.create_default_space().is_none());
}

#[test]
fn failed_pending_profile_deletion_load_prevents_bootstrap_views_and_native_erasure() {
    let store = Arc::new(FakeStore::default());
    store
        .pending_load_failures
        .store(1, std::sync::atomic::Ordering::Release);
    let (mut shell, engine, screen) = setup_with(store);

    shell.handle(Command::Bootstrap);

    assert!(!shell.bootstrapped);
    assert!(shell.windows.focused().is_none());
    assert!(last(&screen).tabs.is_empty());
    let calls = engine.calls();
    assert!(!calls.iter().any(|call| call.starts_with("create ")));
    assert!(!calls.iter().any(|call| call.starts_with("erase-profile ")));
}

#[test]
fn failed_session_load_never_bootstraps_or_overwrites_storage() {
    let store = Arc::new(FakeStore::default());
    *store.load_failed.lock().unwrap() = true;
    let (mut shell, engine, screen) = setup_with(store.clone());

    shell.handle(Command::Bootstrap);
    shell.persist();

    assert!(!shell.bootstrapped);
    assert!(shell.windows.focused().is_none());
    assert!(last(&screen).tabs.is_empty());
    assert!(!store.events.lock().unwrap().contains(&"save"));
    assert!(engine.calls().is_empty());
}

#[test]
fn an_unrestorable_session_is_set_aside_and_the_browser_opens_with_its_profile() {
    let store = Arc::new(FakeStore::default());
    *store.recovery_reason.lock().unwrap() = Some("corrupt authoritative session snapshot".into());
    let profile = ProfileId::from(32_000);
    *store.set_aside.lock().unwrap() = Some(SessionState {
        profiles: vec![PersistedProfile {
            id: profile,
            name: "Personal".into(),
            kind: ProfileKind::Default,
        }],
        ..SessionState::default()
    });
    let (mut shell, _engine, _screen) = setup_with(store);

    shell.handle(Command::Bootstrap);

    assert!(shell.bootstrapped);
    assert!(shell.session_set_aside);
    assert_eq!(shell.windows.focused().unwrap().profile, profile);
}

#[test]
fn recovery_required_never_bootstraps_or_overwrites_storage_when_it_cannot_be_set_aside() {
    let store = Arc::new(FakeStore::default());
    *store.recovery_reason.lock().unwrap() = Some("snapshot is not canonical".into());
    let (mut shell, engine, screen) = setup_with(store.clone());

    shell.handle(Command::Bootstrap);
    shell.persist();

    assert!(!shell.bootstrapped);
    assert!(shell.windows.focused().is_none());
    assert!(last(&screen).tabs.is_empty());
    assert!(!store.events.lock().unwrap().contains(&"save"));
    assert!(engine.calls().is_empty());
}

#[test]
fn degraded_ancillary_profiles_are_recorded_without_blocking_exact_session_bootstrap() {
    let store = Arc::new(FakeStore::default());
    let profile = ProfileId::from(31_000);
    let space = SpaceId::from(31_001);
    *store.saved.lock().unwrap() = Some(SessionState {
        profiles: vec![PersistedProfile {
            id: profile,
            name: "Personal".into(),
            kind: ProfileKind::Default,
        }],
        spaces: vec![PersistedSpace {
            id: space,
            profile,
            name: "Space".into(),
            look: Default::default(),
        }],
        active_space: Some(space),
        ..SessionState::default()
    });
    store.degraded_profiles.lock().unwrap().push(profile);
    let (mut shell, _engine, _screen) = setup_with(store);

    shell.handle(Command::Bootstrap);

    assert!(shell.bootstrapped);
    assert_eq!(
        shell.degraded_storage_profiles,
        std::collections::HashSet::from([profile])
    );
    assert!(shell.profiles.get(profile).is_some());
}

#[test]
fn forged_degraded_profile_report_cannot_bootstrap_an_unrelated_session() {
    let store = Arc::new(FakeStore::default());
    let profile = ProfileId::from(32_000);
    let space = SpaceId::from(32_001);
    *store.saved.lock().unwrap() = Some(SessionState {
        profiles: vec![PersistedProfile {
            id: profile,
            name: "Personal".into(),
            kind: ProfileKind::Default,
        }],
        spaces: vec![PersistedSpace {
            id: space,
            profile,
            name: "Space".into(),
            look: Default::default(),
        }],
        active_space: Some(space),
        ..SessionState::default()
    });
    store
        .degraded_profiles
        .lock()
        .unwrap()
        .push(ProfileId::from(99_999));
    let (mut shell, engine, _screen) = setup_with(store);

    shell.handle(Command::Bootstrap);

    assert!(!shell.bootstrapped);
    assert!(shell.degraded_storage_profiles.is_empty());
    assert!(engine.calls().is_empty());
}

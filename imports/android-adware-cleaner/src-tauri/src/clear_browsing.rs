use adb_bridge::AdbBridge;
use std::thread;
use std::time::Duration;

struct Bounds {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

struct UiNode {
    parent: Option<usize>,
    children: Vec<usize>,
    text: String,
    desc: String,
    id: String,
    class: String,
    clickable: bool,
    checked: bool,
    scrollable: bool,
    bounds: Option<Bounds>,
}

struct Ui {
    nodes: Vec<UiNode>,
}

struct Phone<'a> {
    bridge: &'a AdbBridge,
}

pub fn clear(bridge: &AdbBridge, package: &str) -> Result<(), String> {
    match package {
        "com.android.chrome" | "com.chrome.beta" => clear_chrome(bridge, package),
        "org.mozilla.firefox" => clear_firefox(bridge),
        "com.sec.android.app.sbrowser" => clear_samsung(bridge),
        "com.mi.globalbrowser" => clear_mi(bridge),
        _ => Err(format!("Browser non supportato: {package}")),
    }
}

fn clear_chrome(bridge: &AdbBridge, package: &str) -> Result<(), String> {
    let phone = Phone { bridge };
    phone.ensure(package, "Chrome")?;
    phone.launch(&format!("{package}/com.google.android.apps.chrome.Main"))?;
    let mut expanded = false;
    for _ in 0..12 {
        let ui = phone.dump()?;
        if expanded && chrome_full_page(&ui) {
            break;
        }
        if let Some(more) = ui.by_id_suffix("quick_delete_more_options").or_else(|| ui.labeled(&CHROME_MORE)) {
            phone.tap(&ui, ui.press(more))?;
            expanded = true;
            phone.pause(800);
            continue;
        }
        if ui.nodes.iter().any(|node| node_visible(node) && id_ends(&node.id, "menu_item_text")) {
            let item = ui.labeled(&CHROME_MENU).ok_or("Menu di Chrome aperto, ma senza Elimina dati navigazione")?;
            phone.tap(&ui, ui.press(item))?;
            phone.pause(800);
            continue;
        }
        if chrome_full_page(&ui) {
            phone.key("4")?;
            phone.pause(500);
            continue;
        }
        if let Some(button) = ui.by_id_suffix("menu_button") {
            phone.tap(&ui, button)?;
            phone.pause(700);
            continue;
        }
        if !phone.shows(&ui, package) {
            phone.launch(&format!("{package}/com.google.android.apps.chrome.Main"))?;
            continue;
        }
        phone.key("4")?;
        phone.pause(500);
    }
    let ui = phone.dump()?;
    if !chrome_full_page(&ui) {
        return Err("Non sono arrivato alla schermata Elimina dati navigazione".into());
    }
    chrome_time(&phone)?;
    chrome_boxes(&phone)?;
    chrome_delete(&phone)?;
    Ok(())
}

fn chrome_full_page(ui: &Ui) -> bool {
    let scope = chrome_scope(ui);
    !ui.rows(scope, "checkbox").is_empty() && ui.spinner_text(scope).is_some()
}

fn chrome_scope(ui: &Ui) -> Option<usize> {
    ui.by_id_suffix("preferences_detail").filter(|id| {
        ui.descendants(*id).any(|node| {
            CHROME_TIME.iter().any(|label| norm(&ui.nodes[node].text) == norm(label))
                || id_ends(&ui.nodes[node].id, "checkbox")
        })
    })
}

fn chrome_time(phone: &Phone) -> Result<(), String> {
    let mut ui = phone.dump()?;
    let scope = chrome_scope(&ui);
    if CHROME_ALL.iter().any(|label| norm(label) == norm(&ui.spinner_text(scope).unwrap_or_default())) {
        return Ok(());
    }
    let spinner = scope
        .and_then(|id| ui.by_id_suffix_under(id, "spinner"))
        .or_else(|| scope.and_then(|id| ui.by_id_suffix_under(id, "quick_delete_spinner")))
        .or_else(|| ui.labeled(&CHROME_TIME));
    let spinner = spinner.ok_or("Manca Intervallo di tempo")?;
    phone.tap(&ui, ui.press(spinner))?;
    phone.pause(700);
    for _ in 0..4 {
        ui = phone.dump()?;
        if let Some(option) = ui.nodes.iter().position(|node| {
            node.class == "android.widget.CheckedTextView"
                && node_visible(node)
                && CHROME_ALL.iter().any(|label| norm(label) == norm(&node.text))
        }) {
            phone.tap(&ui, option)?;
            phone.pause(600);
            let again = phone.dump()?;
            let selected = again.spinner_text(chrome_scope(&again)).unwrap_or_default();
            if !CHROME_ALL.iter().any(|label| norm(label) == norm(&selected)) {
                return Err(format!("Intervallo rimasto {selected}"));
            }
            return Ok(());
        }
        if let Some(list) = ui.nodes.iter().position(|node| node.class == "android.widget.ListView" && node_visible(node)) {
            phone.swipe(&ui, list, true)?;
            phone.pause(400);
        } else {
            break;
        }
    }
    Err("Nell'elenco non c'è Dall'inizio".into())
}

fn chrome_boxes(phone: &Phone) -> Result<(), String> {
    let mut seen = Vec::new();
    let mut ui = phone.dump()?;
    let mut previous = String::new();
    for _ in 0..8 {
        let scope = chrome_scope(&ui);
        let rows = ui.rows(scope, "checkbox");
        for row in rows {
            let want = !is_chrome_password(&row.title);
            let key = norm(&row.title);
            if row.checked == want {
                if !seen.iter().any(|item: &String| item == &key) {
                    seen.push(key);
                }
                continue;
            }
            phone.tap(&ui, row.tap)?;
            phone.pause(400);
            ui = phone.dump()?;
            let again = ui.rows(chrome_scope(&ui), "checkbox").into_iter().find(|item| norm(&item.title) == key);
            if again.as_ref().map(|item| item.checked) != Some(want) {
                return Err(format!("Non sono riuscito a impostare {}", row.title));
            }
            if !seen.iter().any(|item: &String| item == &key) {
                seen.push(key);
            }
        }
        let signature = ui.rows(chrome_scope(&ui), "checkbox").iter().map(|row| format!("{}:{}", norm(&row.title), row.checked as u8)).collect::<Vec<_>>().join(",");
        if signature == previous {
            break;
        }
        previous = signature;
        if let Some(scope) = chrome_scope(&ui) {
            if let Some(scroller) = ui.recycler(scope) {
                phone.swipe(&ui, scroller, true)?;
                phone.pause(500);
                ui = phone.dump()?;
                continue;
            }
        }
        break;
    }
    if !seen.iter().any(|title| is_chrome_password(title)) {
        return Err("Casella Password salvate non trovata".into());
    }
    if ui.rows(chrome_scope(&ui), "checkbox").iter().any(|row| is_chrome_password(&row.title) && row.checked) {
        return Err("Password salvate è ancora spuntata".into());
    }
    Ok(())
}

fn chrome_delete(phone: &Phone) -> Result<(), String> {
    let mut ui = phone.dump()?;
    for _ in 0..4 {
        if let Some(button) = ui.by_id_suffix("clear_button") {
            phone.tap(&ui, button)?;
            return chrome_confirm(phone);
        }
        if let Some(scope) = chrome_scope(&ui) {
            if let Some(scroller) = ui.recycler(scope) {
                phone.swipe(&ui, scroller, true)?;
                phone.pause(500);
                ui = phone.dump()?;
                continue;
            }
        }
        break;
    }
    Err("Pulsante Elimina dati non visibile".into())
}

fn chrome_confirm(phone: &Phone) -> Result<(), String> {
    for step in 0..5 {
        phone.pause(700);
        let ui = phone.dump()?;
        if ui.by_id_suffix("clear_button").is_none() {
            return Ok(());
        }
        let extra = ui.by_id_suffix("positive_button").or_else(|| ui.labeled(&CHROME_CONFIRM));
        if let Some(extra) = extra {
            if !text_mentions_password(&ui.nodes[extra].text) {
                phone.tap(&ui, extra)?;
                continue;
            }
        }
        if step == 4 {
            return Err("La schermata di Chrome è ancora aperta".into());
        }
    }
    Ok(())
}

fn clear_firefox(bridge: &AdbBridge) -> Result<(), String> {
    let phone = Phone { bridge };
    phone.ensure("org.mozilla.firefox", "Firefox")?;
    phone.launch("org.mozilla.firefox/.App")?;
    for _ in 0..16 {
        let ui = phone.dump()?;
        if !phone.shows(&ui, "org.mozilla.firefox") {
            phone.launch("org.mozilla.firefox/.App")?;
            continue;
        }
        if ui.by_id_suffix("delete_data").is_some() {
            return firefox_finish(&phone);
        }
        if let Some(clear) = labeled_row(&ui, &FIREFOX_CLEAR, false) {
            phone.tap(&ui, clear)?;
            phone.pause(700);
            continue;
        }
        if let Some(settings) = labeled_row(&ui, &FIREFOX_SETTINGS, true) {
            phone.tap(&ui, settings)?;
            phone.pause(700);
            continue;
        }
        if let Some(menu) = labeled_row(&ui, &FIREFOX_MENU, false) {
            phone.tap(&ui, menu)?;
            phone.pause(700);
            continue;
        }
        if !phone.scroll(&ui) {
            phone.key("4")?;
        }
        phone.pause(400);
    }
    Err("Non sono arrivato a Elimina dati di navigazione".into())
}

fn firefox_finish(phone: &Phone) -> Result<(), String> {
    let mut ui = phone.dump()?;
    for _ in 0..6 {
        for row in ui.rows(None, "checkbox") {
            if row.checked {
                continue;
            }
            phone.tap(&ui, row.tap)?;
            phone.pause(400);
            ui = phone.dump()?;
        }
        if let Some(button) = ui.by_id_suffix("delete_data").filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, button)?;
            return firefox_confirm(phone);
        }
        if !phone.scroll(&ui) {
            break;
        }
        phone.pause(400);
        ui = phone.dump()?;
    }
    Err("Pulsante Elimina dati di navigazione non visibile".into())
}

fn firefox_confirm(phone: &Phone) -> Result<(), String> {
    for _ in 0..6 {
        phone.pause(500);
        let ui = phone.dump()?;
        if let Some(button) = firefox_confirm_button(&ui).filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, button)?;
            return Ok(());
        }
    }
    Err("Conferma Elimina non visibile".into())
}

fn firefox_confirm_button(ui: &Ui) -> Option<usize> {
    ui.nodes.iter().position(|node| {
        node_visible(node)
            && id_ends(&node.id, "button1")
            && FIREFOX_CONFIRM.iter().any(|label| norm(label) == norm(&node.text))
            && !text_mentions_password(&node.text)
    })
}

fn clear_samsung(bridge: &AdbBridge) -> Result<(), String> {
    let phone = Phone { bridge };
    phone.ensure("com.sec.android.app.sbrowser", "Samsung Internet")?;
    phone.launch("com.sec.android.app.sbrowser/.SBrowserMainActivity")?;
    for _ in 0..18 {
        let ui = phone.dump()?;
        if !phone.shows(&ui, "com.sec.android.app.sbrowser") {
            phone.launch("com.sec.android.app.sbrowser/.SBrowserMainActivity")?;
            continue;
        }
        if ui.by_id_suffix("delete_button").is_some() && ui.by_id_suffix("clear_browsing_data_spinner").is_some() {
            samsung_time(&phone)?;
            samsung_boxes(&phone)?;
            let ui = phone.dump()?;
            let button = ui.by_id_suffix("delete_button").ok_or("Pulsante Elimina dati non visibile")?;
            phone.tap(&ui, button)?;
            return Ok(());
        }
        if let Some(clear) = labeled_row(&ui, &SAMSUNG_CLEAR, false) {
            phone.tap(&ui, clear)?;
            phone.pause(700);
            continue;
        }
        if let Some(personal) = labeled_row(&ui, &SAMSUNG_PERSONAL, false) {
            phone.tap(&ui, personal)?;
            phone.pause(700);
            continue;
        }
        if let Some(settings) = labeled_row(&ui, &SAMSUNG_SETTINGS, false) {
            phone.tap(&ui, settings)?;
            phone.pause(700);
            continue;
        }
        if let Some(menu) = ui.by_id_suffix("action_more").filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, menu)?;
            phone.pause(700);
            continue;
        }
        if !phone.scroll(&ui) {
            phone.key("4")?;
        }
        phone.pause(400);
    }
    Err("Non sono arrivato a Elimina dati navigazione".into())
}

fn samsung_time(phone: &Phone) -> Result<(), String> {
    let ui = phone.dump()?;
    let Some(spinner) = ui.by_id_suffix("clear_browsing_data_spinner") else {
        return Err("Manca l'intervallo di tempo".into());
    };
    let current = ui.nodes[spinner].children.iter().find_map(|child| {
        let text = &ui.nodes[*child].text;
        if text.is_empty() { None } else { Some(text.clone()) }
    }).unwrap_or_default();
    if SAMSUNG_ALL.iter().any(|label| norm(label) == norm(&current)) {
        return Ok(());
    }
    phone.tap(&ui, ui.press(spinner))?;
    phone.pause(700);
    for _ in 0..4 {
        let ui = phone.dump()?;
        if let Some(option) = ui.labeled(&SAMSUNG_ALL).filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, ui.press(option))?;
            phone.pause(500);
            return Ok(());
        }
        if !phone.scroll(&ui) {
            break;
        }
        phone.pause(400);
    }
    Err("Nell'elenco non c'è tutti gli intervalli di tempo".into())
}

fn samsung_boxes(phone: &Phone) -> Result<(), String> {
    let mut seen = Vec::new();
    let mut ui = phone.dump()?;
    let mut previous = String::new();
    for step in 0..6 {
        for row in ui.rows(None, "check") {
            let key = norm(&row.title);
            if !seen.iter().any(|item: &String| item == &key) {
                seen.push(key);
            }
        }
        let signature = seen.join("|");
        if signature == previous && step > 0 {
            break;
        }
        previous = signature;
        if !phone.scroll_toward(&ui, step == 0)? {
            break;
        }
        phone.pause(400);
        ui = phone.dump()?;
    }
    if !seen.iter().any(|title| is_samsung_password(title)) {
        return Err("Voce Password non trovata".into());
    }
    phone.scroll_toward(&ui, true)?;
    phone.pause(400);
    ui = phone.dump()?;
    for step in 0..6 {
        let mut changed = false;
        for row in ui.rows(None, "check") {
            let want = !is_samsung_password(&row.title);
            if row.checked == want {
                continue;
            }
            phone.tap(&ui, row.tap)?;
            phone.pause(400);
            ui = phone.dump()?;
            changed = true;
        }
        if !changed && step > 0 {
            break;
        }
        if !phone.scroll(&ui) {
            break;
        }
        phone.pause(400);
        ui = phone.dump()?;
    }
    if ui.rows(None, "check").iter().any(|row| is_samsung_password(&row.title) && row.checked) {
        return Err("Password è ancora spuntata".into());
    }
    Ok(())
}

fn clear_mi(bridge: &AdbBridge) -> Result<(), String> {
    let phone = Phone { bridge };
    phone.ensure("com.mi.globalbrowser", "Mi Browser")?;
    phone.launch("com.mi.globalbrowser/.Main")?;
    let mut opened = false;
    for _ in 0..16 {
        let ui = phone.dump()?;
        if !phone.shows(&ui, "com.mi.globalbrowser") {
            phone.launch("com.mi.globalbrowser/.Main")?;
            continue;
        }
        let known = mi_rows(&ui).into_iter().filter(|row| MI_DATA.iter().any(|label| norm(label) == norm(&row.title)) || is_mi_password(&row.title)).count();
        if known >= 2 {
            opened = true;
            break;
        }
        if let Some(clear) = labeled_row(&ui, &MI_CLEAR, false) {
            phone.tap(&ui, clear)?;
            phone.pause(700);
            continue;
        }
        if let Some(settings) = ui.by_id_suffix("action_menu_setting_button").filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, settings)?;
            phone.pause(700);
            continue;
        }
        if let Some(profile) = ui.by_id_suffix("action_my").filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, profile)?;
            phone.pause(700);
            continue;
        }
        if !phone.scroll(&ui) {
            phone.key("4")?;
        }
        phone.pause(400);
    }
    if !opened {
        return Err("Non sono arrivato a Cancella dati".into());
    }
    let mut seen: Vec<(String, String)> = Vec::new();
    let mut ui = phone.dump()?;
    let mut previous = String::new();
    for step in 0..6 {
        for row in mi_rows(&ui) {
            let key = norm(&row.title);
            if !seen.iter().any(|(item, _)| item == &key) {
                seen.push((key, row.title));
            }
        }
        let signature = seen.iter().map(|(key, _)| key.clone()).collect::<Vec<_>>().join("|");
        if signature == previous && step > 0 {
            break;
        }
        previous = signature;
        if !phone.scroll_toward(&ui, step == 0)? {
            break;
        }
        phone.pause(400);
        ui = phone.dump()?;
    }
    if !seen.iter().any(|(key, _)| is_mi_password(key)) {
        return Err("Voce Password salvate non trovata".into());
    }
    let pending: Vec<String> = seen.into_iter().filter(|(key, _)| !is_mi_password(key)).map(|(_, title)| title).collect();
    for title in pending {
        let mut tapped = false;
        for _ in 0..6 {
            ui = phone.dump()?;
            if let Some(row) = mi_rows(&ui).into_iter().find(|row| norm(&row.title) == norm(&title) && on_screen(&ui, row.tap)) {
                phone.tap(&ui, row.tap)?;
                mi_ok(&phone)?;
                tapped = true;
                break;
            }
            phone.scroll(&ui);
            phone.pause(400);
        }
        if !tapped {
            return Err(format!("Voce non trovata: {title}"));
        }
    }
    Ok(())
}

fn mi_ok(phone: &Phone) -> Result<(), String> {
    for _ in 0..4 {
        let ui = phone.dump()?;
        if let Some(ok) = ui.by_id_suffix("button1").filter(|id| on_screen(&ui, *id)) {
            phone.tap(&ui, ok)?;
            phone.pause(700);
            return Ok(());
        }
        phone.pause(300);
    }
    Err("Conferma OK non trovata".into())
}

struct Row {
    title: String,
    checked: bool,
    tap: usize,
}

impl Ui {
    fn descendants(&self, root: usize) -> impl Iterator<Item = usize> + '_ {
        let mut pending = self.nodes[root].children.clone();
        let mut out = Vec::new();
        while let Some(id) = pending.pop() {
            out.push(id);
            pending.extend(self.nodes[id].children.iter().copied());
        }
        out.into_iter()
    }

    fn by_id_suffix(&self, suffix: &str) -> Option<usize> {
        self.nodes.iter().position(|node| node_visible(node) && id_ends(&node.id, suffix))
    }

    fn by_id_suffix_under(&self, root: usize, suffix: &str) -> Option<usize> {
        self.descendants(root).find(|id| node_visible(&self.nodes[*id]) && id_ends(&self.nodes[*id].id, suffix))
    }

    fn labeled(&self, labels: &[&str]) -> Option<usize> {
        self.nodes.iter().position(|node| {
            node_visible(node) && labels.iter().any(|label| norm(label) == norm(&node.text) || norm(label) == norm(&node.desc))
        })
    }

    fn press(&self, id: usize) -> usize {
        if self.nodes[id].clickable && node_visible(&self.nodes[id]) {
            return id;
        }
        let mut current = self.nodes[id].parent;
        while let Some(parent) = current {
            if self.nodes[parent].clickable && node_visible(&self.nodes[parent]) {
                return parent;
            }
            current = self.nodes[parent].parent;
        }
        id
    }

    fn spinner_text(&self, scope: Option<usize>) -> Option<String> {
        let spinner = scope.and_then(|id| self.by_id_suffix_under(id, "spinner"))?;
        self.nodes[spinner].children.iter().find_map(|child| {
            let text = &self.nodes[*child].text;
            if text.is_empty() { None } else { Some(text.clone()) }
        })
    }

    fn recycler(&self, scope: usize) -> Option<usize> {
        self.descendants(scope).filter(|id| {
            let node = &self.nodes[*id];
            node.scrollable && node_visible(node) && !node.class.ends_with("Spinner") && id_ends(&node.id, "recycler_view")
        }).max_by_key(|id| bounds_height(&self.nodes[*id]))
    }

    fn rows(&self, scope: Option<usize>, suffix: &str) -> Vec<Row> {
        let ids: Vec<usize> = match scope {
            Some(scope) => self.descendants(scope).collect(),
            None => (0..self.nodes.len()).collect(),
        };
        let mut rows = Vec::new();
        for id in ids {
            let node = &self.nodes[id];
            if !id_ends(&node.id, suffix) || !node.class.ends_with("CheckBox") || !node_visible(node) {
                continue;
            }
            let mut row = id;
            while let Some(parent) = self.nodes[row].parent {
                if self.nodes[row].clickable {
                    break;
                }
                row = parent;
            }
            let title = self.descendants(row).find_map(|child| {
                let node = &self.nodes[child];
                if id_ends(&node.id, "title") && !node.text.is_empty() {
                    Some(node.text.clone())
                } else {
                    None
                }
            });
            let Some(title) = title else { continue };
            rows.push(Row { title, checked: self.nodes[id].checked, tap: row });
        }
        rows
    }
}

fn mi_rows(ui: &Ui) -> Vec<Row> {
    let mut rows = Vec::new();
    for id in 0..ui.nodes.len() {
        let node = &ui.nodes[id];
        if !id_ends(&node.id, "title") || node.text.is_empty() || id_ends(&node.id, "action_bar_title_expand") {
            continue;
        }
        let tap = row_target(ui, id);
        let Some(tap) = tap else { continue };
        if id_ends(&ui.nodes[tap].id, "button") {
            continue;
        }
        rows.push(Row { title: node.text.clone(), checked: false, tap });
    }
    rows
}

fn labeled_row(ui: &Ui, labels: &[&str], desc_only: bool) -> Option<usize> {
    ui.nodes.iter().enumerate().find_map(|(id, node)| {
        if !node_visible(node) {
            return None;
        }
        if node.id.contains("toolbar") {
            return None;
        }
        let text_match = !desc_only && labels.iter().any(|label| norm(label) == norm(&node.text));
        let desc_match = labels.iter().any(|label| norm(label) == norm(&node.desc));
        if !text_match && !desc_match {
            return None;
        }
        let target = row_target(ui, id)?;
        if !on_screen(ui, target) || ui.nodes[target].id.contains("toolbar") {
            return None;
        }
        Some(target)
    })
}

fn row_target(ui: &Ui, id: usize) -> Option<usize> {
    let target = ui.press(id);
    let height = bounds_height(&ui.nodes[target])?;
    if height > 500 { None } else { Some(target) }
}

fn on_screen(ui: &Ui, id: usize) -> bool {
    let Some(point) = center(&ui.nodes[id]) else { return false };
    let Some(screen) = ui.nodes.iter().find_map(|node| node.bounds.as_ref()) else { return false };
    point.0 >= screen.left && point.0 <= screen.right && point.1 >= screen.top + 8 && point.1 <= screen.bottom - 8
}

fn center(node: &UiNode) -> Option<(i32, i32)> {
    let area = node.bounds.as_ref()?;
    if area.right <= area.left || area.bottom <= area.top {
        return None;
    }
    Some(((area.left + area.right) / 2, (area.top + area.bottom) / 2))
}

fn bounds_height(node: &UiNode) -> Option<i32> {
    node.bounds.as_ref().map(|area| area.bottom - area.top)
}

fn node_visible(node: &UiNode) -> bool {
    node.bounds.as_ref().is_some_and(|area| area.right > area.left && area.bottom > area.top)
}

fn id_ends(id: &str, suffix: &str) -> bool {
    id.ends_with(&format!("/{suffix}")) || id.ends_with(&format!(":id/{suffix}"))
}

fn norm(value: &str) -> String {
    value.replace(['\u{2019}', '\u{2018}'], "'").split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

pub fn is_chrome_password(title: &str) -> bool {
    CHROME_PASSWORDS.iter().any(|label| norm(label) == norm(title))
}

pub fn is_samsung_password(title: &str) -> bool {
    SAMSUNG_PASSWORDS.iter().any(|label| norm(label) == norm(title))
}

pub fn is_mi_password(title: &str) -> bool {
    MI_PASSWORDS.iter().any(|label| norm(label) == norm(title))
}

fn text_mentions_password(text: &str) -> bool {
    let value = norm(text);
    value.contains("password") || CHROME_PASSWORDS.iter().any(|label| value.contains(&norm(label)))
}

impl Phone<'_> {
    fn ensure(&self, package: &str, label: &str) -> Result<(), String> {
        let out = self.bridge.shell(&format!("pm path {package}")).map_err(|e| e.to_string())?;
        if !out.contains("package:") {
            return Err(format!("{label} non installato"));
        }
        Ok(())
    }

    fn launch(&self, component: &str) -> Result<(), String> {
        self.bridge.shell("input keyevent 224").map_err(|e| e.to_string())?;
        self.bridge.shell("wm dismiss-keyguard").ok();
        let out = self.bridge.shell(&format!("am start -n {component}")).map_err(|e| e.to_string())?;
        if out.contains("Error:") {
            return Err(format!("Impossibile aprire {component}"));
        }
        self.pause(800);
        Ok(())
    }

    fn shows(&self, ui: &Ui, package: &str) -> bool {
        ui.nodes.iter().any(|node| node.id.starts_with(package))
    }

    fn dump(&self) -> Result<Ui, String> {
        let mut last = String::from("Dump dell'interfaccia non riuscito");
        for _ in 0..3 {
            match self.bridge.shell("uiautomator dump /sdcard/uidump.xml") {
                Ok(_) => match self.bridge.shell("cat /sdcard/uidump.xml") {
                    Ok(xml) if xml.contains("<hierarchy") => return Ok(parse_ui(&xml)),
                    Ok(_) => last = "Dump dell'interfaccia vuoto".into(),
                    Err(error) => last = error.to_string(),
                },
                Err(error) => last = error.to_string(),
            }
            self.pause(400);
        }
        Err(last)
    }

    fn tap(&self, ui: &Ui, id: usize) -> Result<(), String> {
        let (x, y) = center(&ui.nodes[id]).ok_or("Controllo senza area visibile")?;
        self.bridge.shell(&format!("input tap {x} {y}")).map(|_| ()).map_err(|e| e.to_string())
    }

    fn swipe(&self, ui: &Ui, id: usize, toward_end: bool) -> Result<(), String> {
        let Some(area) = ui.nodes[id].bounds.as_ref() else { return Ok(()) };
        let x = (area.left + area.right) / 2;
        let start = area.top + (area.bottom - area.top) * 72 / 100;
        let end = area.top + (area.bottom - area.top) * 28 / 100;
        let (y1, y2) = if toward_end { (start, end) } else { (end, start) };
        self.bridge.shell(&format!("input swipe {x} {y1} {x} {y2} 350")).map(|_| ()).map_err(|e| e.to_string())
    }

    fn scroll(&self, ui: &Ui) -> bool {
        self.scroll_toward(ui, false).unwrap_or(false)
    }

    fn scroll_toward(&self, ui: &Ui, toward_start: bool) -> Result<bool, String> {
        let Some(scroller) = ui.nodes.iter().enumerate().filter(|(_, node)| node.scrollable && node_visible(node) && !node.class.ends_with("Spinner")).max_by_key(|(_, node)| bounds_height(node).unwrap_or(0)).map(|(id, _)| id) else {
            return Ok(false);
        };
        self.swipe(ui, scroller, !toward_start)?;
        Ok(true)
    }

    fn key(&self, code: &str) -> Result<(), String> {
        self.bridge.shell(&format!("input keyevent {code}")).map(|_| ()).map_err(|e| e.to_string())
    }

    fn pause(&self, ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }
}

fn parse_ui(xml: &str) -> Ui {
    let mut nodes = vec![UiNode {
        parent: None,
        children: Vec::new(),
        text: String::new(),
        desc: String::new(),
        id: String::new(),
        class: String::new(),
        clickable: false,
        checked: false,
        scrollable: false,
        bounds: None,
    }];
    let mut stack = vec![0usize];
    let bytes = xml.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if xml[index..].starts_with("<node") {
            let start = index + 5;
            let end = xml[start..].find('>').map(|pos| start + pos).unwrap_or(xml.len());
            let tag = &xml[start..end];
            let close = tag.ends_with('/');
            let body = tag.trim_end_matches('/').trim();
            let id = nodes.len();
            let parent = *stack.last().unwrap();
            nodes.push(parse_node(body, Some(parent)));
            nodes[parent].children.push(id);
            if !close {
                stack.push(id);
            }
            index = end + 1;
        } else if xml[index..].starts_with("</node>") {
            if stack.len() > 1 {
                stack.pop();
            }
            index += "</node>".len();
        } else {
            index += 1;
        }
    }
    Ui { nodes }
}

fn parse_node(body: &str, parent: Option<usize>) -> UiNode {
    let attr = |name: &str| -> String {
        let key = format!("{name}=\"");
        let Some(start) = body.find(&key) else { return String::new() };
        let rest = &body[start + key.len()..];
        rest.split('"').next().unwrap_or("").to_string()
    };
    let bounds = attr("bounds");
    UiNode {
        parent,
        children: Vec::new(),
        text: decode(&attr("text")),
        desc: decode(&attr("content-desc")),
        id: attr("resource-id"),
        class: attr("class"),
        clickable: attr("clickable") == "true",
        checked: attr("checked") == "true",
        scrollable: attr("scrollable") == "true",
        bounds: parse_bounds(&bounds),
    }
}

fn parse_bounds(value: &str) -> Option<Bounds> {
    let nums: Vec<i32> = value.split(|ch: char| !ch.is_ascii_digit()).filter(|part| !part.is_empty()).filter_map(|part| part.parse().ok()).collect();
    if nums.len() < 4 {
        return None;
    }
    Some(Bounds { left: nums[0], top: nums[1], right: nums[2], bottom: nums[3] })
}

fn decode(value: &str) -> String {
    value.replace("&quot;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

const CHROME_MENU: &[&str] = &["Elimina dati navigazione", "Delete browsing data", "Supprimer les données de navigation", "Borrar datos de navegación", "Browserdaten löschen", "Șterge datele de navigare", "Fshi të dhënat e shfletimit", "حذف بيانات التصفّح", "删除浏览数据", "Видалити дані вебперегляду"];
const CHROME_MORE: &[&str] = &["Altre opzioni", "More options", "Plus d'options", "Más opciones", "Weitere Optionen", "Mai multe opțiuni", "Më shumë opsione", "خيارات إضافية", "更多选项", "Інші опції"];
const CHROME_ALL: &[&str] = &["Dall'inizio", "All time", "Depuis le début", "Desde siempre", "Gesamte Zeit", "Dintotdeauna", "Gjatë të gjithë kohës", "جميع الأوقات", "时间不限", "Увесь час"];
const CHROME_PASSWORDS: &[&str] = &["Password salvate", "Saved passwords", "Mots de passe enregistrés", "Contraseñas guardadas", "Gespeicherte Passwörter", "Parole salvate", "Fjalëkalimet e ruajtura", "كلمات المرور المحفوظة", "已保存的密码", "Збережені паролі"];
const CHROME_TIME: &[&str] = &["Intervallo di tempo", "Time range", "Période", "Tiempo", "Zeitraum", "Interval de timp", "Shtrirja kohore", "النطاق الزمني", "时间范围", "Період часу"];
const CHROME_CONFIRM: &[&str] = &["Elimina", "Elimina dati", "Elimina comunque", "Delete", "Delete data", "Supprimer", "Supprimer les données", "Eliminar", "Eliminar datos", "Löschen", "Daten löschen", "Șterge", "Șterge datele", "Fshi", "Fshi të dhënat", "حذف", "حذف البيانات", "删除", "删除数据", "Видалити", "Видалити дані"];
const FIREFOX_MENU: &[&str] = &["Altre opzioni", "More options", "Plus d'options", "Más opciones", "Weitere Optionen", "Mai multe opțiuni", "Më tepër mundësi", "خيارات إضافية", "更多选项", "Додатково"];
const FIREFOX_SETTINGS: &[&str] = &["Impostazioni", "Settings", "Paramètres", "Ajustes", "Einstellungen", "Setări", "Rregullime", "الإعدادات", "设置", "Налаштування"];
const FIREFOX_CLEAR: &[&str] = &["Elimina dati di navigazione", "Delete browsing data", "Supprimer les données de navigation", "Eliminar datos del navegador", "Eliminar datos de navegación", "Browser-Daten löschen", "Șterge datele de navigare", "Fshi të dhëna shfletimi", "حذف بيانات التصفح", "احذف بيانات التصفح", "删除浏览数据", "Видалити дані перегляду"];
const FIREFOX_CONFIRM: &[&str] = &["Elimina", "Delete", "Supprimer", "Eliminar", "Löschen", "Șterge", "Fshije", "احذف", "删除", "Видалити"];
const SAMSUNG_SETTINGS: &[&str] = &["Impostazioni", "Settings", "Paramètres", "Ajustes", "Einstellungen", "Setări", "الإعدادات", "设置", "Налаштування"];
const SAMSUNG_PERSONAL: &[&str] = &["Dati di navigazione personali", "Personal browsing data", "Données navigation personnelles", "Datos de navegación personales", "Persönliche Browserdaten", "Date de navigare personale", "بيانات التصفح الشخصية", "个人浏览数据", "Особисті дані про перегляд"];
const SAMSUNG_CLEAR: &[&str] = &["Elimina dati navigazione", "Delete browsing data", "Suppression données de navigation", "Eliminar datos de navegación", "Browserdaten löschen", "Ștergere date de navigare", "حذف بيانات التصفح", "删除浏览数据", "Видалення даних перегляду"];
const SAMSUNG_ALL: &[&str] = &["Tutti gli intervalli di tempo", "All time", "Toutes périodes", "Todo", "Alles", "Tot timpul", "كل الوقت", "始终", "За весь час"];
const SAMSUNG_PASSWORDS: &[&str] = &["Password", "Passwords", "Mots de passe", "Contraseñas", "Passwörter", "Parole", "كلمات المرور", "密码", "Паролі"];
const MI_CLEAR: &[&str] = &["Cancella dati", "Clear data", "Effacer les données", "Borrar datos", "Daten löschen", "Şterge date", "مسح البيانات", "清除数据", "Очистити дані"];
const MI_DATA: &[&str] = &["Cache", "Dati modulo", "Form data", "Données de formulaire", "Datos de formularios", "Formulardaten", "Date din formular", "بيانات النماذج", "清除表单数据", "Дані форм", "Cookies", "ملفات تعريف الارتباط", "清除所有 Cookie 数据", "Файли cookie", "Cookie-uri", "Cronologia", "History", "Historique", "Historial", "Verlauf", "Istoric", "السجل", "清除历史记录", "Історія", "Impostazioni dei siti", "Site settings", "Paramètres du site", "Ajustes del sitio", "Website-Einstellungen", "Setări site", "إعدادات الموقع", "清除网站设置", "Налаштування сайту"];
const MI_PASSWORDS: &[&str] = &["Password salvate", "Saved passwords", "Mots de passe enregistrés", "Contraseñas guardadas", "Gespeicherte Passwörter", "Parole salvate", "كلمة المرور المحفوظة", "清除已保存的密码", "Збережені паролі"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chrome_passwords_stay_off() {
        assert!(is_chrome_password("Saved passwords"));
        assert!(is_chrome_password("Mots de passe enregistrés"));
        assert!(!is_chrome_password("Browsing history"));
    }

    #[test]
    fn samsung_and_mi_passwords_stay_off() {
        assert!(is_samsung_password("Passwords"));
        assert!(is_samsung_password("Mots de passe"));
        assert!(!is_samsung_password("Browsing history"));
        assert!(is_mi_password("Saved passwords"));
        assert!(is_mi_password("Mots de passe enregistrés"));
        assert!(!is_mi_password("Cache"));
    }

    #[test]
    fn firefox_confirm_is_the_allow_button() {
        let ui = parse_ui(r#"<hierarchy><node class="android.widget.FrameLayout" bounds="[28,828][1052,1599]"><node text="Annulla" resource-id="android:id/button2" class="android.widget.Button" clickable="true" bounds="[526,1226][718,1352]" /><node text="Elimina" resource-id="android:id/button1" class="android.widget.Button" clickable="true" bounds="[739,1226][926,1352]" /></node></hierarchy>"#);
        let button = firefox_confirm_button(&ui).unwrap();
        assert_eq!(ui.nodes[button].text, "Elimina");
    }

    #[test]
    fn firefox_menu_button_is_tappable() {
        let ui = parse_ui(r#"<hierarchy><node resource-id="org.mozilla.firefox:id/composable_toolbar" class="android.view.View" clickable="false" bounds="[0,93][1080,261]"><node class="android.widget.Button" content-desc="Altre opzioni" clickable="true" bounds="[955,115][1080,241]" /></node></hierarchy>"#);
        assert!(labeled_row(&ui, &FIREFOX_MENU, false).is_some());
    }

    #[test]
    fn error_keeps_the_reason() {
        let error = "Voce Password non trovata".to_string();
        assert!(error.contains("Password non trovata"));
    }

    #[test]
    #[ignore]
    fn device_installed_browsers() {
        let bridge = AdbBridge::new().expect("adb");
        for package in [
            "com.android.chrome",
            "com.chrome.beta",
            "org.mozilla.firefox",
            "com.sec.android.app.sbrowser",
            "com.mi.globalbrowser",
        ] {
            let installed = bridge
                .shell(&format!("pm path {package}"))
                .ok()
                .is_some_and(|out| out.contains("package:"));
            if installed {
                clear(&bridge, package).unwrap_or_else(|error| panic!("{package}: {error}"));
            }
        }
    }
}

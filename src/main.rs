#![windows_subsystem = "windows"]
//! ΑΑΔΕ – Αναζήτηση Βασικών Στοιχείων Μητρώου (RgWsPublic2), native Win32 GUI (native-windows-gui).

use native_windows_gui as nwg;
use nwg::{Event, EventData};
use serde_json::{Map, Value};
use std::{cell::RefCell, rc::Rc, time::Duration};

const URL: &str = "https://www1.gsis.gr/wsaade/RgWsPublic2/RgWsPublic2";
const MIN_W: i32 = 780;
const MIN_H: i32 = 540;

// ---------------------------------------------------------------- ΑΑΔΕ

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn query(user: &str, pass: &str, afm: &str) -> Result<String, String> {
    let body = format!(
        r#"<env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope"
 xmlns:ns1="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
 xmlns:ns2="http://rgwspublic2/RgWsPublic2Service" xmlns:ns3="http://rgwspublic2/RgWsPublic2">
<env:Header><ns1:Security><ns1:UsernameToken>
<ns1:Username>{}</ns1:Username><ns1:Password>{}</ns1:Password>
</ns1:UsernameToken></ns1:Security></env:Header>
<env:Body><ns2:rgWsPublic2AfmMethod><ns2:INPUT_REC>
<ns3:afm_called_by/><ns3:afm_called_for>{}</ns3:afm_called_for>
</ns2:INPUT_REC></ns2:rgWsPublic2AfmMethod></env:Body></env:Envelope>"#,
        esc(user),
        esc(pass),
        afm
    );
    let resp = ureq::post(URL)
        .timeout(Duration::from_secs(30))
        .set("Content-Type", "application/soap+xml; charset=utf-8")
        .send_string(&body);
    match resp {
        Ok(r) => r.into_string().map_err(|e| e.to_string()),
        // Τα SOAP faults έρχονται με HTTP 500 – θέλουμε το σώμα τους
        Err(ureq::Error::Status(_, r)) => r.into_string().map_err(|e| e.to_string()),
        Err(e) => Err(format!("Αποτυχία σύνδεσης με την ΑΑΔΕ:\n{e}")),
    }
}

/// XML -> JSON: φύλλα γίνονται string/null, επαναλαμβανόμενα tags (και `item`) γίνονται πίνακες.
fn to_json(node: roxmltree::Node) -> Value {
    let kids: Vec<_> = node.children().filter(|n| n.is_element()).collect();
    if kids.is_empty() {
        return match node.text().map(str::trim) {
            Some(t) if !t.is_empty() => Value::String(t.into()),
            _ => Value::Null,
        };
    }
    let mut m = Map::new();
    for k in kids {
        let name = k.tag_name().name().to_string();
        let v = to_json(k);
        match m.get_mut(&name) {
            Some(Value::Array(a)) => a.push(v),
            Some(old) => *old = Value::Array(vec![old.take(), v]),
            None => {
                let v = if name == "item" { Value::Array(vec![v]) } else { v };
                m.insert(name, v);
            }
        }
    }
    Value::Object(m)
}

fn parse(xml: &str) -> Result<Value, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| format!("Μη αναμενόμενη απάντηση:\n{xml}"))?;
    let find = |name: &str| doc.descendants().find(|n| n.tag_name().name() == name);
    let node = find("rg_ws_public2_result_rtType")
        .or_else(|| find("Body").and_then(|b| b.first_element_child()))
        .ok_or("Κενή απάντηση")?;
    Ok(to_json(node))
}

/// Πρώτη τιμή-κείμενο με αυτό το κλειδί, οπουδήποτε μέσα στο `v`.
fn find<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    match v {
        Value::Object(m) => m.get(k).and_then(Value::as_str).or_else(|| m.values().find_map(|x| find(x, k))),
        Value::Array(a) => a.iter().find_map(|x| find(x, k)),
        _ => None,
    }
}

fn label(k: &str) -> &str {
    match k {
        "call_seq_id" => "Αριθμός κλήσης",
        "error_code" => "Κωδικός σφάλματος",
        "error_descr" => "Περιγραφή σφάλματος",
        "token_username" => "Ειδικός κωδικός",
        "token_afm" => "ΑΦΜ χρήστη",
        "token_afm_fullname" => "Επωνυμία χρήστη",
        "afm_called_by" => "ΑΦΜ καλούντος",
        "afm_called_by_fullname" => "Επωνυμία καλούντος",
        "as_on_date" => "Ημ/νία αναζήτησης",
        "afm" => "ΑΦΜ",
        "doy" => "Κωδικός ΔΟΥ",
        "doy_descr" => "ΔΟΥ",
        "i_ni_flag_descr" => "Είδος προσώπου",
        "deactivation_flag" => "Κωδικός κατάστασης",
        "deactivation_flag_descr" => "Κατάσταση ΑΦΜ",
        "firm_flag_descr" => "Ιδιότητα",
        "onomasia" => "Επωνυμία",
        "commer_title" => "Διακριτικός τίτλος",
        "legal_status_descr" => "Νομική μορφή",
        "postal_address" => "Οδός",
        "postal_address_no" => "Αριθμός",
        "postal_zip_code" => "Τ.Κ.",
        "postal_area_description" => "Περιοχή",
        "regist_date" => "Ημ/νία έναρξης",
        "stop_date" => "Ημ/νία διακοπής",
        "normal_vat_system_flag" => "Κανονικό καθεστώς ΦΠΑ",
        _ => k,
    }
}

fn creds() -> Option<keyring::Entry> {
    keyring::Entry::new("aade-afm-lookup", "credentials").ok()
}

// ---------------------------------------------------------------- φόρμα

/// (tab, γραμμή, στήλη 0=αριστερά 1=δεξιά 2=όλο το πλάτος, πεδίο)
const FIELDS: &[(usize, usize, u8, &str)] = &[
    (0, 0, 2, "onomasia"),
    (0, 1, 2, "commer_title"),
    (0, 2, 0, "afm"),
    (0, 2, 1, "legal_status_descr"),
    (0, 3, 0, "doy_descr"),
    (0, 3, 1, "doy"),
    (0, 4, 0, "deactivation_flag_descr"),
    (0, 4, 1, "deactivation_flag"),
    (0, 5, 0, "firm_flag_descr"),
    (0, 5, 1, "i_ni_flag_descr"),
    (0, 6, 0, "postal_address"),
    (0, 6, 1, "postal_address_no"),
    (0, 7, 0, "postal_zip_code"),
    (0, 7, 1, "postal_area_description"),
    (0, 8, 0, "regist_date"),
    (0, 8, 1, "stop_date"),
    (0, 9, 0, "normal_vat_system_flag"),
    (2, 0, 0, "call_seq_id"),
    (2, 0, 1, "as_on_date"),
    (2, 1, 0, "token_username"),
    (2, 2, 0, "token_afm"),
    (2, 2, 1, "token_afm_fullname"),
    (2, 3, 0, "afm_called_by"),
    (2, 3, 1, "afm_called_by_fullname"),
    (2, 4, 0, "error_code"),
    (2, 5, 2, "error_descr"),
];
const MULTI: &[&str] = &["onomasia", "error_descr"]; // πολλές γραμμές με αναδίπλωση
const LABEL_W: i32 = 165;
const ROW_H: i32 = 26;
const MULTI_H: i32 = 46;
const GAP: i32 = 8;

enum Val {
    Line(nwg::TextInput),
    Multi(nwg::TextBox),
    Check(nwg::CheckBox),
}

struct Field {
    tab: usize,
    row: usize,
    col: u8,
    key: &'static str,
    label: nwg::Label,
    val: Val,
}

#[derive(Default)]
struct Ui {
    window: nwg::Window,
    embed: nwg::EmbedResource,
    icon: nwg::Icon,
    l_user: nwg::Label,
    user: nwg::TextInput,
    l_pass: nwg::Label,
    pass: nwg::TextInput,
    save: nwg::Button,
    l_afm: nwg::Label,
    afm: nwg::TextInput,
    search: nwg::Button,
    export: nwg::Button,
    tabs: nwg::TabsContainer,
    tab: [nwg::Tab; 3],
    acts: nwg::TextBox,
    status: nwg::StatusBar,
    fields: Vec<Field>,
    last: RefCell<Option<Value>>,
}

fn build() -> Result<Rc<Ui>, nwg::NwgError> {
    let mut ui = Ui::default();
    nwg::EmbedResource::builder().build(&mut ui.embed)?;
    nwg::Icon::builder().source_embed(Some(&ui.embed)).source_embed_id(1).build(&mut ui.icon)?;
    nwg::Window::builder()
        .size((MIN_W, MIN_H))
        .center(true)
        .title("ΑΑΔΕ – Αναζήτηση Βασικών Στοιχείων Μητρώου")
        .icon(Some(&ui.icon))
        .build(&mut ui.window)?;
    let w = &ui.window;

    let cap = |l: &mut nwg::Label, t: &str| {
        nwg::Label::builder().text(t).parent(w).h_align(nwg::HTextAlign::Right).build(l)
    };
    cap(&mut ui.l_user, "Κωδικός:")?;
    nwg::TextInput::builder().parent(w).build(&mut ui.user)?;
    cap(&mut ui.l_pass, "Συνθηματικό:")?;
    nwg::TextInput::builder().parent(w).password(Some('●')).build(&mut ui.pass)?;
    nwg::Button::builder().text("Αποθήκευση").parent(w).build(&mut ui.save)?;
    cap(&mut ui.l_afm, "ΑΦΜ:")?;
    nwg::TextInput::builder().parent(w).limit(9).build(&mut ui.afm)?;
    nwg::Button::builder().text("Αναζήτηση").parent(w).build(&mut ui.search)?;
    nwg::Button::builder().text("Εξαγωγή JSON").parent(w).enabled(false).build(&mut ui.export)?;

    nwg::TabsContainer::builder().parent(w).build(&mut ui.tabs)?;
    for (t, name) in ui.tab.iter_mut().zip(["Βασικά στοιχεία", "Δραστηριότητες", "Στοιχεία κλήσης"]) {
        nwg::Tab::builder().text(name).parent(&ui.tabs).build(t)?;
    }

    for &(tab, row, col, key) in FIELDS {
        let parent = &ui.tab[tab];
        let mut label = nwg::Label::default();
        // λευκό φόντο, όπως η σελίδα του tab
        nwg::Label::builder()
            .text(&label_of(key))
            .parent(parent)
            .background_color(Some([255, 255, 255]))
            .build(&mut label)?;
        let val = if key == "normal_vat_system_flag" {
            let mut c = nwg::CheckBox::default();
            nwg::CheckBox::builder()
                .text("")
                .parent(parent)
                .background_color(Some([255, 255, 255]))
                .build(&mut c)?;
            Val::Check(c)
        } else if MULTI.contains(&key) {
            let mut t = nwg::TextBox::default();
            nwg::TextBox::builder()
                .parent(parent)
                .readonly(true)
                .flags(nwg::TextBoxFlags::VISIBLE | nwg::TextBoxFlags::TAB_STOP)
                .build(&mut t)?;
            Val::Multi(t)
        } else {
            let mut t = nwg::TextInput::default();
            nwg::TextInput::builder().parent(parent).readonly(true).build(&mut t)?;
            Val::Line(t)
        };
        ui.fields.push(Field { tab, row, col, key, label, val });
    }

    // Δραστηριότητες: κείμενο με αναδίπλωση (ο πίνακας των Windows κόβει τις μεγάλες περιγραφές)
    nwg::TextBox::builder()
        .parent(&ui.tab[1])
        .readonly(true)
        .flags(nwg::TextBoxFlags::VISIBLE | nwg::TextBoxFlags::VSCROLL | nwg::TextBoxFlags::TAB_STOP)
        .build(&mut ui.acts)?;

    nwg::StatusBar::builder().parent(w).text(" Συμπληρώστε ΑΦΜ και πατήστε Αναζήτηση.").build(&mut ui.status)?;

    // Φόρτωση αποθηκευμένων στοιχείων από τον Windows Credential Manager
    if let Some(s) = creds().and_then(|e| e.get_password().ok()) {
        if let Some((u, p)) = s.split_once('\n') {
            ui.user.set_text(u);
            ui.pass.set_text(p);
        }
    }
    Ok(Rc::new(ui))
}

fn label_of(k: &str) -> String {
    format!("{}:", label(k))
}

impl Ui {
    /// Θέσεις controls για το τρέχον μέγεθος παραθύρου.
    fn layout(&self) {
        let (w, h) = self.window.size();
        let (w, h) = (w as i32, h as i32);
        self.l_user.place(8, 15, 80, 22);
        self.user.place(94, 12, 210, 26);
        self.l_pass.place(312, 15, 92, 22);
        self.pass.place(410, 12, (w - 410 - 132).max(120), 26);
        self.save.place(w - 122, 11, 110, 28);
        self.l_afm.place(8, 49, 80, 22);
        self.afm.place(94, 46, 210, 26);
        self.search.place(312, 45, 130, 28);
        self.export.place(450, 45, 130, 28);

        let top = 84;
        let status_h = 26;
        let tabs_h = (h - top - status_h - 8).max(100);
        self.tabs.place(8, top, w - 16, tabs_h);
        // εσωτερικό tab ≈ container μείον κεφαλίδα καρτελών και περιθώρια
        let (tw, th) = (w - 16 - 8, tabs_h - 34);

        // Πεδία: δύο στήλες (ετικέτα + τιμή), ή μία σε όλο το πλάτος
        let half = (tw - 3 * GAP) / 2;
        for f in &self.fields {
            let mut y = GAP + 4;
            for r in 0..f.row {
                let multi = self.fields.iter().any(|g| g.tab == f.tab && g.row == r && MULTI.contains(&g.key));
                y += if multi { MULTI_H } else { ROW_H } + GAP;
            }
            let (x, cw) = match f.col {
                0 => (GAP, half),
                1 => (2 * GAP + half, half),
                _ => (GAP, tw - 2 * GAP),
            };
            let vh = if MULTI.contains(&f.key) { MULTI_H } else { ROW_H };
            f.label.place(x, y + 4, LABEL_W, 20);
            f.val.place(x + LABEL_W, y, cw - LABEL_W, vh);
        }
        self.acts.place(GAP, GAP, tw - 2 * GAP, th - 2 * GAP);
    }

    fn fill(&self, json: &Value) {
        let basic = json.get("basic_rec").unwrap_or(json);
        for f in &self.fields {
            let v = find(if f.tab == 0 { basic } else { json }, f.key).unwrap_or("");
            match &f.val {
                Val::Line(t) => t.set_text(v),
                Val::Multi(t) => t.set_text(v),
                Val::Check(c) => {
                    c.set_check_state(if v == "Y" { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });
                    c.set_text(match v { "Y" => "Ναι", "N" => "Όχι", x => x });
                }
            }
        }
        self.acts.set_text(&acts_text(json));
    }

    fn do_search(&self) {
        let afm = self.afm.text();
        let (user, pass) = (self.user.text(), self.pass.text());
        if afm.len() != 9 || !afm.bytes().all(|b| b.is_ascii_digit()) {
            nwg::modal_error_message(&self.window, "ΑΦΜ", "Το ΑΦΜ πρέπει να έχει 9 ψηφία.");
            return;
        }
        if user.trim().is_empty() || pass.is_empty() {
            nwg::modal_error_message(&self.window, "Στοιχεία", "Συμπληρώστε κωδικό και συνθηματικό.");
            return;
        }
        self.status.set_text(0, " Αναζήτηση…");
        // ponytail: blocking κλήση στο UI thread (~1-2s), thread αν γίνει ενοχλητικό
        match query(user.trim(), &pass, &afm).and_then(|x| parse(&x)) {
            Ok(json) => {
                self.fill(&json);
                match json.pointer("/error_rec/error_descr").and_then(Value::as_str) {
                    Some(d) => {
                        self.status.set_text(0, &format!(" Σφάλμα: {d}"));
                        nwg::modal_error_message(&self.window, "ΑΑΔΕ", d);
                    }
                    None => {
                        let name = find(&json, "onomasia").unwrap_or("");
                        self.status.set_text(0, &format!(" {afm}: {name}"));
                    }
                }
                *self.last.borrow_mut() = Some(json);
                self.export.set_enabled(true);
                self.tabs.set_selected_tab(0);
            }
            Err(e) => {
                self.status.set_text(0, " Αποτυχία αναζήτησης.");
                nwg::modal_error_message(&self.window, "Σφάλμα", &e);
            }
        }
    }

    fn do_save(&self) {
        let res = creds().ok_or("keyring".to_string()).and_then(|e| {
            e.set_password(&format!("{}\n{}", self.user.text().trim(), self.pass.text()))
                .map_err(|e| e.to_string())
        });
        match res {
            Ok(_) => nwg::modal_info_message(&self.window, "Αποθήκευση", "Τα στοιχεία αποθηκεύτηκαν."),
            Err(e) => nwg::modal_error_message(&self.window, "Αποθήκευση", &format!("Αποτυχία: {e}")),
        };
    }

    fn do_export(&self) {
        let Some(json) = self.last.borrow().clone() else { return };
        let afm = json.get("basic_rec").and_then(|b| find(b, "afm")).unwrap_or("").to_string();
        let afm = if afm.is_empty() { self.afm.text() } else { afm };
        let Some(path) = save_dialog(&self.window, &format!("{afm}.json")) else { return };
        if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap()) {
            nwg::modal_error_message(&self.window, "Εξαγωγή", &format!("Αποτυχία εγγραφής: {e}"));
        } else {
            self.status.set_text(0, &format!(" Αποθηκεύτηκε: {}", path.display()));
        }
    }
}

/// Κείμενο δραστηριοτήτων: «ΚΑΔ · Είδος» και από κάτω ολόκληρη η περιγραφή.
fn acts_text(json: &Value) -> String {
    let Some(Value::Array(items)) = json.pointer("/firm_act_tab/item") else {
        return String::new();
    };
    items
        .iter()
        .map(|it| {
            let g = |k| find(it, k).unwrap_or("");
            format!("ΚΑΔ {}  ·  {}\r\n{}", g("firm_act_code"), g("firm_act_kind_descr"), g("firm_act_descr"))
        })
        .collect::<Vec<_>>()
        .join("\r\n\r\n")
}

/// Παράθυρο «Αποθήκευση ως» των Windows με προτεινόμενο όνομα αρχείου
/// (το FileDialog του NWG δεν υποστηρίζει προτεινόμενο όνομα).
fn save_dialog(owner: &nwg::Window, name: &str) -> Option<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::commdlg::*;
    let wide = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    let mut file = [0u16; 1024];
    for (i, c) in name.encode_utf16().take(file.len() - 1).enumerate() {
        file[i] = c;
    }
    let filter = wide("JSON (*.json)\0*.json\0Όλα τα αρχεία (*.*)\0*.*\0\0");
    let ext = wide("json\0");
    let title = wide("Εξαγωγή JSON\0");
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner.handle.hwnd().unwrap_or(std::ptr::null_mut());
    ofn.lpstrFilter = filter.as_ptr();
    ofn.lpstrFile = file.as_mut_ptr();
    ofn.nMaxFile = file.len() as u32;
    ofn.lpstrDefExt = ext.as_ptr(); // προσθέτει .json αν λείπει
    ofn.lpstrTitle = title.as_ptr();
    ofn.Flags = OFN_EXPLORER | OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if unsafe { GetSaveFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(std::ffi::OsString::from_wide(&file[..len]).into())
}

/// Θέση + μέγεθος σε λογικές μονάδες (το NWG κάνει την κλιμάκωση DPI).
trait Place {
    fn place(&self, x: i32, y: i32, w: i32, h: i32);
}
macro_rules! place_impl {
    ($($t:ty),*) => {$(
        impl Place for $t {
            fn place(&self, x: i32, y: i32, w: i32, h: i32) {
                self.set_position(x, y);
                self.set_size(w.max(0) as u32, h.max(0) as u32);
            }
        }
    )*};
}
place_impl!(nwg::Label, nwg::TextInput, nwg::TextBox, nwg::Button, nwg::CheckBox, nwg::TabsContainer);

impl Place for Val {
    fn place(&self, x: i32, y: i32, w: i32, h: i32) {
        match self {
            Val::Line(t) => t.place(x, y, w, h),
            Val::Multi(t) => t.place(x, y, w, h),
            Val::Check(c) => c.place(x, y, w, h),
        }
    }
}

fn main() {
    nwg::init().expect("Αποτυχία αρχικοποίησης GUI");
    let _ = nwg::Font::set_global_family("Segoe UI");
    let ui = build().expect("Αποτυχία δημιουργίας παραθύρου");
    ui.layout();

    let weak = Rc::downgrade(&ui);
    let handler = nwg::full_bind_event_handler(&ui.window.handle, move |evt, data, handle| {
        let Some(ui) = weak.upgrade() else { return };
        match evt {
            Event::OnWindowClose if handle == ui.window.handle => nwg::stop_thread_dispatch(),
            Event::OnResize | Event::OnWindowMaximize if handle == ui.window.handle => ui.layout(),
            Event::OnMinMaxInfo => {
                if let EventData::OnMinMaxInfo(info) = &data {
                    info.set_min_size(MIN_W, MIN_H);
                }
            }
            Event::OnButtonClick if handle == ui.search.handle => ui.do_search(),
            Event::OnButtonClick if handle == ui.save.handle => ui.do_save(),
            Event::OnButtonClick if handle == ui.export.handle => ui.do_export(),
            Event::OnKeyEnter if handle == ui.afm.handle || handle == ui.pass.handle => ui.do_search(),
            // checkbox ΦΠΑ μόνο για ανάγνωση: επαναφορά της τιμής της ΑΑΔΕ
            Event::OnButtonClick => {
                for f in &ui.fields {
                    if let Val::Check(c) = &f.val {
                        if handle == c.handle {
                            ui.fill(ui.last.borrow().as_ref().unwrap_or(&Value::Null));
                        }
                    }
                }
            }
            _ => {}
        }
    });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_to_json() {
        let xml = r#"<e:Envelope xmlns:e="x"><e:Body><r><rg_ws_public2_result_rtType>
            <basic_rec><afm>800686740</afm><onomasia>ΑΒΓ</onomasia><x/></basic_rec>
            <firm_act_tab><item><c>1</c></item></firm_act_tab>
            <dup>a</dup><dup>b</dup></rg_ws_public2_result_rtType></r></e:Body></e:Envelope>"#;
        let j = parse(xml).unwrap();
        assert_eq!(j["basic_rec"]["afm"], "800686740");
        assert_eq!(j["basic_rec"]["x"], Value::Null);
        assert_eq!(j["firm_act_tab"]["item"][0]["c"], "1");
        assert_eq!(j["dup"][1], "b");
        assert_eq!(find(&j, "onomasia"), Some("ΑΒΓ"));
        let acts = serde_json::json!({"firm_act_tab": {"item": [
            {"firm_act_code": "1", "firm_act_descr": "ΑΑ", "firm_act_kind_descr": "ΚΥΡΙΑ"},
            {"firm_act_code": "2", "firm_act_descr": "ΒΒ", "firm_act_kind_descr": "ΔΕΥΤΕΡΕΥΟΥΣΑ"}]}});
        assert_eq!(acts_text(&acts), "ΚΑΔ 1  ·  ΚΥΡΙΑ\r\nΑΑ\r\n\r\nΚΑΔ 2  ·  ΔΕΥΤΕΡΕΥΟΥΣΑ\r\nΒΒ");
        assert_eq!(esc("a&<b>"), "a&amp;&lt;b&gt;");
    }
}

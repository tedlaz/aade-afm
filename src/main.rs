#![windows_subsystem = "windows"]
//! ΑΑΔΕ – Αναζήτηση Βασικών Στοιχείων Μητρώου (RgWsPublic2), native Win32 GUI (native-windows-gui).

use native_windows_gui as nwg;
use nwg::{Event, EventData};
use serde_json::{Map, Value};
use std::{cell::RefCell, rc::Rc, time::Duration};

const URL: &str = "https://www1.gsis.gr/wsaade/RgWsPublic2/RgWsPublic2";
const MIN_W: i32 = 780;
const MIN_H: i32 = 570;

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

/// VIES (Ευρωπαϊκή Επιτροπή): χωρίς κωδικούς, αλλά μόνο επωνυμία/διεύθυνση και μόνο για ΑΦΜ
/// ενεργά στις ενδοκοινοτικές. None σε σφάλμα ή μη έγκυρο ΑΦΜ.
fn vies(cc: &str, afm: &str) -> Option<Value> {
    let url = format!("https://ec.europa.eu/taxation_customs/vies/rest-api/ms/{cc}/vat/{afm}");
    let body = ureq::get(&url).timeout(Duration::from_secs(15)).set("Accept", "application/json").call().ok()?.into_string().ok()?;
    let raw: Value = serde_json::from_str(&body).ok()?;
    if raw.get("isValid").or(raw.get("valid")) != Some(&Value::Bool(true)) {
        return None;
    }
    // "---" = το κράτος-μέλος δεν κοινοποιεί το στοιχείο
    let s = |k| raw.get(k).and_then(Value::as_str).map(str::trim).filter(|v| *v != "---").unwrap_or("");
    let (name, title) = s("name").split_once("||").unwrap_or((s("name"), ""));
    let [street, no, zip, area] = split_address(s("address"));
    let basic = serde_json::json!({
        "afm": if cc == "EL" { afm.to_string() } else { format!("{cc}{afm}") }, "onomasia": name.trim(), "commer_title": title.trim(),
        "postal_address": street, "postal_address_no": no, "postal_zip_code": zip, "postal_area_description": area,
    });
    Some(basic)
}

const MODES: [&str; 3] = ["Ελλάδα – VIES", "Ελλάδα – ΑΑΔΕ", "Ενδοκοινοτική – VIES"];
const MODE_GR_VIES: usize = 0;
const MODE_GR_AADE: usize = 1;
const MODE_EU_VIES: usize = 2;

/// Κράτη-μέλη VIES εκτός Ελλάδας (XI = Βόρεια Ιρλανδία).
const COUNTRIES: &[(&str, &str)] = &[
    ("AT", "Αυστρία"), ("BE", "Βέλγιο"), ("BG", "Βουλγαρία"), ("CY", "Κύπρος"), ("CZ", "Τσεχία"),
    ("DE", "Γερμανία"), ("DK", "Δανία"), ("EE", "Εσθονία"), ("ES", "Ισπανία"), ("FI", "Φινλανδία"),
    ("FR", "Γαλλία"), ("HR", "Κροατία"), ("HU", "Ουγγαρία"), ("IE", "Ιρλανδία"), ("IT", "Ιταλία"),
    ("LT", "Λιθουανία"), ("LU", "Λουξεμβούργο"), ("LV", "Λετονία"), ("MT", "Μάλτα"), ("NL", "Ολλανδία"),
    ("PL", "Πολωνία"), ("PT", "Πορτογαλία"), ("RO", "Ρουμανία"), ("SE", "Σουηδία"), ("SI", "Σλοβενία"),
    ("SK", "Σλοβακία"), ("XI", "Βόρεια Ιρλανδία"),
];

/// «de 123.456.789» με χώρα DE → «123456789» (χωρίς σημεία στίξης και πρόθεμα χώρας).
fn clean_vat(cc: &str, input: &str) -> String {
    let s = input.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_uppercase();
    let alias = if cc == "EL" { "GR" } else { cc };
    s.strip_prefix(cc).or(s.strip_prefix(alias)).unwrap_or(&s).to_string()
}

/// Διεύθυνση VIES «ΑΙΟΛΟΥ 86        10559 - ΑΘΗΝΑ» → [οδός, αριθμός, Τ.Κ., περιοχή].
/// Πιάνει και τη συνήθη μορφή ΕΕ «VIA RICHINI 6\n20122 MILANO MI».
/// ponytail: απλοί κανόνες (π.χ. όχι «AT-5330 Fuschl»)· ό,τι δεν ταιριάζει μένει ολόκληρο στην οδό.
fn split_address(a: &str) -> [String; 4] {
    let mut parts: Vec<&str> =
        a.lines().flat_map(|l| l.split("   ")).map(|p| p.trim_matches([' ', ','])).filter(|p| !p.is_empty()).collect();
    let (mut zip, mut area) = ("", "");
    if parts.len() > 1 && parts.last().is_some_and(|p| p.starts_with(|c: char| c.is_ascii_digit())) {
        let last = parts.pop().unwrap();
        let (z, c) = last.split_once(' ').unwrap_or((last, ""));
        (zip, area) = (z, c.trim_start_matches(['-', ' ']));
    }
    let street = parts.join(", ");
    let (street, no) = match street.rsplit_once(' ') {
        Some((s, n)) if n.starts_with(|c: char| c.is_ascii_digit()) => (s.to_string(), n.to_string()),
        _ => (street.clone(), String::new()),
    };
    [street, no, zip.into(), area.into()]
}

fn creds() -> Option<keyring::Entry> {
    keyring::Entry::new("aade-afm-lookup", "credentials").ok()
}

// ---------------------------------------------------------------- φόρμα

/// Σελίδες με πεδία: οι δύο καρτέλες VIES και οι υποκαρτέλες της ΑΑΔΕ.
const P_GR: usize = 0;
const P_EU: usize = 1;
const P_BASIC: usize = 2;
const P_CALL: usize = 3;

/// (γραμμή, στήλη 0=αριστερά 1=δεξιά 2=όλο το πλάτος, πεδίο) — ό,τι δίνει το VIES
const VIES_FIELDS: &[(usize, u8, &str)] = &[
    (0, 2, "onomasia"),
    (1, 2, "commer_title"),
    (2, 0, "afm"),
    (3, 0, "postal_address"),
    (3, 1, "postal_address_no"),
    (4, 0, "postal_zip_code"),
    (4, 1, "postal_area_description"),
];
/// (σελίδα, γραμμή, στήλη, πεδίο) — ΑΑΔΕ
const AADE_FIELDS: &[(usize, usize, u8, &str)] = &[
    (P_BASIC, 0, 2, "onomasia"),
    (P_BASIC, 1, 2, "commer_title"),
    (P_BASIC, 2, 0, "afm"),
    (P_BASIC, 2, 1, "legal_status_descr"),
    (P_BASIC, 3, 0, "doy_descr"),
    (P_BASIC, 3, 1, "doy"),
    (P_BASIC, 4, 0, "deactivation_flag_descr"),
    (P_BASIC, 4, 1, "deactivation_flag"),
    (P_BASIC, 5, 0, "firm_flag_descr"),
    (P_BASIC, 5, 1, "i_ni_flag_descr"),
    (P_BASIC, 6, 0, "postal_address"),
    (P_BASIC, 6, 1, "postal_address_no"),
    (P_BASIC, 7, 0, "postal_zip_code"),
    (P_BASIC, 7, 1, "postal_area_description"),
    (P_BASIC, 8, 0, "regist_date"),
    (P_BASIC, 8, 1, "stop_date"),
    (P_BASIC, 9, 0, "normal_vat_system_flag"),
    (P_CALL, 0, 0, "call_seq_id"),
    (P_CALL, 0, 1, "as_on_date"),
    (P_CALL, 1, 0, "token_username"),
    (P_CALL, 2, 0, "token_afm"),
    (P_CALL, 2, 1, "token_afm_fullname"),
    (P_CALL, 3, 0, "afm_called_by"),
    (P_CALL, 3, 1, "afm_called_by_fullname"),
    (P_CALL, 4, 0, "error_code"),
    (P_CALL, 5, 2, "error_descr"),
];
const MULTI: &[&str] = &["onomasia", "error_descr"]; // πολλές γραμμές με αναδίπλωση
const LABEL_W: i32 = 165;
const ROW_H: i32 = 26;
const MULTI_H: i32 = 46;
const GAP: i32 = 8;
const BAR_H: i32 = 40; // γραμμή αναζήτησης στην κορυφή κάθε καρτέλας

enum Val {
    Line(nwg::TextInput),
    Multi(nwg::TextBox),
    Check(nwg::CheckBox),
}

struct Field {
    page: usize,
    row: usize,
    col: u8,
    key: &'static str,
    label: nwg::Label,
    val: Val,
}

/// ΑΦΜ + Αναζήτηση, μία ανά καρτέλα αναζήτησης.
#[derive(Default)]
struct Bar {
    l_afm: nwg::Label,
    afm: nwg::TextInput,
    search: nwg::Button,
    note: nwg::RichLabel,
}

#[derive(Default)]
struct Ui {
    window: nwg::Window,
    embed: nwg::EmbedResource,
    icon: nwg::Icon,
    tabs: nwg::TabsContainer,
    tab: [nwg::Tab; 4], // MODES + Παράμετροι
    bars: [Bar; 3],
    country: nwg::ComboBox<String>,
    itabs: nwg::TabsContainer, // υποκαρτέλες ΑΑΔΕ
    itab: [nwg::Tab; 3],
    acts: nwg::TextBox,
    l_user: nwg::Label,
    user: nwg::TextInput,
    l_pass: nwg::Label,
    pass: nwg::TextInput,
    fallback: nwg::CheckBox,
    save: nwg::Button,
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
        .title(concat!("ΑΑΔΕ – Αναζήτηση Βασικών Στοιχείων Μητρώου  v", env!("CARGO_PKG_VERSION")))
        .icon(Some(&ui.icon))
        .build(&mut ui.window)?;

    nwg::TabsContainer::builder().parent(&ui.window).build(&mut ui.tabs)?;
    for (t, name) in ui.tab.iter_mut().zip(MODES.into_iter().chain(["Παράμετροι"])) {
        nwg::Tab::builder().text(name).parent(&ui.tabs).build(t)?;
    }
    let cap = |l: &mut nwg::Label, t: &str, p: &nwg::Tab, bg: [u8; 3]| {
        nwg::Label::builder().text(t).parent(p).background_color(Some(bg)).build(l)
    };

    // Γραμμή αναζήτησης σε κάθε καρτέλα (η ενδοκοινοτική έχει και επιλογή χώρας)
    nwg::ComboBox::builder()
        .parent(&ui.tab[MODE_EU_VIES])
        .collection(COUNTRIES.iter().map(|(c, n)| format!("{c} – {n}")).collect())
        .selected_index(Some(0))
        .build(&mut ui.country)?;
    for (i, (b, p)) in ui.bars.iter_mut().zip(&ui.tab).enumerate() {
        cap(&mut b.l_afm, "ΑΦΜ:", p, TAB_BG[i])?;
        nwg::TextInput::builder().parent(p).limit(20).build(&mut b.afm)?;
        nwg::Button::builder().text("Αναζήτηση").parent(p).build(&mut b.search)?;
        // RichLabel: το απλό Label του NWG δεν έχει χρώμα κειμένου
        let aade = i == MODE_GR_AADE;
        let note = if aade { "Αναζήτηση με κωδικούς ΑΑΔΕ" } else { "Ανώνυμη αναζήτηση" };
        nwg::RichLabel::builder().text(note).parent(p).background_color(Some(TAB_BG[i])).build(&mut b.note)?;
        let color = if aade { [200, 0, 0] } else { [100, 100, 100] };
        b.note.set_char_format(0..b.note.len(), &nwg::CharFormat {
            text_color: Some(color),
            height: Some(200), // 10pt σε twips, όπως οι υπόλοιπες ετικέτες
            font_face_name: Some("Segoe UI".into()),
            ..Default::default()
        });
    }

    nwg::TabsContainer::builder().parent(&ui.tab[MODE_GR_AADE]).build(&mut ui.itabs)?;
    for (t, name) in ui.itab.iter_mut().zip(["Βασικά στοιχεία", "Δραστηριότητες", "Στοιχεία κλήσης"]) {
        nwg::Tab::builder().text(name).parent(&ui.itabs).build(t)?;
    }

    // Παράμετροι: κωδικοί ΑΑΔΕ
    let p = &ui.tab[3];
    cap(&mut ui.l_user, "Κωδικός ΑΑΔΕ:", p, TAB_BG[3])?;
    nwg::TextInput::builder().parent(p).build(&mut ui.user)?;
    cap(&mut ui.l_pass, "Συνθηματικό ΑΑΔΕ:", p, TAB_BG[3])?;
    nwg::TextInput::builder().parent(p).password(Some('●')).build(&mut ui.pass)?;
    nwg::CheckBox::builder()
        .text("«Ελλάδα – VIES»: αν το ΑΦΜ δεν βρεθεί στο VIES, αναζήτηση στην ΑΑΔΕ")
        .parent(p)
        .background_color(Some(TAB_BG[3]))
        .build(&mut ui.fallback)?;
    nwg::Button::builder().text("Αποθήκευση").parent(p).build(&mut ui.save)?;

    let specs = [P_GR, P_EU]
        .into_iter()
        .flat_map(|p| VIES_FIELDS.iter().map(move |&(r, c, k)| (p, r, c, k)))
        .chain(AADE_FIELDS.iter().copied());
    for (page, row, col, key) in specs {
        let parent = match page {
            P_GR => &ui.tab[MODE_GR_VIES],
            P_EU => &ui.tab[MODE_EU_VIES],
            P_BASIC => &ui.itab[0],
            _ => &ui.itab[2],
        };
        let bg = TAB_BG[match page { P_GR => MODE_GR_VIES, P_EU => MODE_EU_VIES, _ => MODE_GR_AADE }];
        let mut label = nwg::Label::default();
        cap(&mut label, &label_of(key), parent, bg)?;
        let val = if key == "normal_vat_system_flag" {
            let mut c = nwg::CheckBox::default();
            nwg::CheckBox::builder().text("").parent(parent).background_color(Some(bg)).build(&mut c)?;
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
        ui.fields.push(Field { page, row, col, key, label, val });
    }

    // Δραστηριότητες: κείμενο με αναδίπλωση (ο πίνακας των Windows κόβει τις μεγάλες περιγραφές)
    nwg::TextBox::builder()
        .parent(&ui.itab[1])
        .readonly(true)
        .flags(nwg::TextBoxFlags::VISIBLE | nwg::TextBoxFlags::VSCROLL | nwg::TextBoxFlags::TAB_STOP)
        .build(&mut ui.acts)?;

    nwg::StatusBar::builder().parent(&ui.window).text(" Συμπληρώστε ΑΦΜ και πατήστε Αναζήτηση.").build(&mut ui.status)?;

    // Χρώμα σελίδων: στο τέλος, ώστε ο handler να προηγείται αυτών των controls
    for (t, &bg) in ui.tab.iter().zip(&TAB_BG) {
        paint_bg(t, bg);
    }
    for t in &ui.itab {
        paint_bg(t, TAB_BG[MODE_GR_AADE]);
    }

    // Φόρτωση αποθηκευμένων στοιχείων από τον Windows Credential Manager
    if let Some(s) = creds().and_then(|e| e.get_password().ok()) {
        // ponytail: η επιλογή fallback αποθηκεύεται ως 3η γραμμή μαζί με τους κωδικούς
        let mut it = s.splitn(3, '\n');
        ui.user.set_text(it.next().unwrap_or(""));
        ui.pass.set_text(it.next().unwrap_or(""));
        if it.next() == Some("1") {
            ui.fallback.set_check_state(nwg::CheckBoxState::Checked);
        }
    }
    Ok(Rc::new(ui))
}

/// Χρώμα φόντου ανά καρτέλα (MODES + Παράμετροι), για να φαίνεται αμέσως πού βρισκόμαστε.
const TAB_BG: [[u8; 3]; 4] = [[226, 238, 252], [255, 241, 222], [228, 245, 230], [255, 255, 255]];

/// Βάφει το φόντο μιας σελίδας tab (το Tab του NWG δεν έχει background_color), καθώς και
/// ετικέτες/checkbox με διάφανο φόντο κειμένου (το NWG δίνει μόνο brush, το κείμενο μένει σε λευκό).
/// Τα read-only πεδία (Edit) κρατούν το γκρι τους.
fn paint_bg(tab: &nwg::Tab, [r, g, b]: [u8; 3]) {
    use winapi::um::wingdi::{CreateSolidBrush, RGB, SetBkMode, TRANSPARENT};
    use winapi::um::winuser::{FillRect, GetClassNameW, GetClientRect, WM_CTLCOLORSTATIC, WM_ERASEBKGND};
    let brush = unsafe { CreateSolidBrush(RGB(r, g, b)) } as usize; // ζει όσο η εφαρμογή
    let _ = nwg::bind_raw_event_handler(&tab.handle, 0x10000, move |hwnd, msg, w, l| unsafe {
        match msg {
            WM_ERASEBKGND => {
                let mut rc = std::mem::zeroed();
                GetClientRect(hwnd, &mut rc);
                FillRect(w as _, &rc, brush as _);
                Some(1)
            }
            WM_CTLCOLORSTATIC => {
                let mut cls = [0u16; 16];
                let n = GetClassNameW(l as _, cls.as_mut_ptr(), cls.len() as i32) as usize;
                if String::from_utf16_lossy(&cls[..n]) == "Edit" {
                    return None;
                }
                SetBkMode(w as _, TRANSPARENT as i32);
                Some(brush as _)
            }
            _ => None,
        }
    });
}

fn label_of(k: &str) -> String {
    format!("{}:", label(k))
}

impl Ui {
    /// Θέσεις controls για το τρέχον μέγεθος παραθύρου.
    fn layout(&self) {
        let (w, h) = self.window.size();
        let (w, h) = (w as i32, h as i32);
        let status_h = 26;
        let tabs_h = (h - GAP - status_h - GAP).max(100);
        self.tabs.place(GAP, GAP, w - 2 * GAP, tabs_h);
        // εσωτερικό tab ≈ container μείον κεφαλίδα καρτελών και περιθώρια
        let (tw, th) = (w - 2 * GAP - 8, tabs_h - 34);

        // Γραμμή αναζήτησης
        for (i, b) in self.bars.iter().enumerate() {
            let x = if i == MODE_EU_VIES { GAP + 180 + GAP } else { GAP };
            b.l_afm.place(x, GAP + 7, 40, 20);
            b.afm.place(x + 44, GAP + 4, 150, ROW_H);
            b.search.place(x + 44 + 150 + GAP, GAP + 3, 120, 28);
            let nx = x + 44 + 150 + GAP + 120 + 2 * GAP;
            b.note.place(nx, GAP + 9, tw - nx - GAP, 20);
        }
        self.country.place(GAP, GAP + 4, 180, ROW_H);

        // Υποκαρτέλες ΑΑΔΕ κάτω από τη γραμμή αναζήτησης
        let (iw, ih) = (tw - 2 * GAP, th - BAR_H - 2 * GAP);
        self.itabs.place(GAP, BAR_H + GAP, iw, ih);
        let (iw, ih) = (iw - 8, ih - 34);
        self.acts.place(GAP, GAP, iw - 2 * GAP, ih - 2 * GAP);

        // Πεδία: δύο στήλες (ετικέτα + τιμή), ή μία σε όλο το πλάτος
        for f in &self.fields {
            let (top, pw) = if f.page == P_GR || f.page == P_EU { (BAR_H, tw) } else { (0, iw) };
            let mut y = top + GAP + 4;
            for r in 0..f.row {
                let multi = self.fields.iter().any(|g| g.page == f.page && g.row == r && MULTI.contains(&g.key));
                y += if multi { MULTI_H } else { ROW_H } + GAP;
            }
            let half = (pw - 3 * GAP) / 2;
            let (x, cw) = match f.col {
                0 => (GAP, half),
                1 => (2 * GAP + half, half),
                _ => (GAP, pw - 2 * GAP),
            };
            let vh = if MULTI.contains(&f.key) { MULTI_H } else { ROW_H };
            f.label.place(x, y + 4, LABEL_W, 20);
            f.val.place(x + LABEL_W, y, cw - LABEL_W, vh);
        }

        let y = |r: i32| GAP + 4 + r * (ROW_H + GAP);
        self.l_user.place(GAP, y(0) + 4, LABEL_W, 20);
        self.user.place(GAP + LABEL_W, y(0), 260, ROW_H);
        self.l_pass.place(GAP, y(1) + 4, LABEL_W, 20);
        self.pass.place(GAP + LABEL_W, y(1), 260, ROW_H);
        self.fallback.place(GAP, y(2), tw - 2 * GAP, ROW_H);
        self.save.place(GAP + LABEL_W, y(3) + GAP, 130, 28);
    }

    /// Γεμίζει τα πεδία των σελίδων `pages` (Null = καθάρισμα).
    fn fill(&self, json: &Value, pages: &[usize]) {
        let basic = json.get("basic_rec").unwrap_or(json);
        for f in self.fields.iter().filter(|f| pages.contains(&f.page)) {
            let v = find(if f.page == P_CALL { json } else { basic }, f.key).unwrap_or("");
            match &f.val {
                Val::Line(t) => t.set_text(v),
                Val::Multi(t) => t.set_text(v),
                Val::Check(c) => {
                    c.set_check_state(if v == "Y" { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });
                    c.set_text(match v { "Y" => "Ναι", "N" => "Όχι", x => x });
                }
            }
        }
    }

    fn has_creds(&self) -> bool {
        !self.user.text().trim().is_empty() && !self.pass.text().is_empty()
    }

    fn do_search(&self, mode: usize) {
        let cc = if mode == MODE_EU_VIES { COUNTRIES[self.country.selection().unwrap_or(0)].0 } else { "EL" };
        let afm = clean_vat(cc, &self.bars[mode].afm.text());
        if cc == "EL" && (afm.len() != 9 || !afm.bytes().all(|b| b.is_ascii_digit())) {
            nwg::modal_error_message(&self.window, "ΑΦΜ", "Το ελληνικό ΑΦΜ πρέπει να έχει 9 ψηφία.");
            return;
        }
        if afm.is_empty() {
            nwg::modal_error_message(&self.window, "ΑΦΜ", "Συμπληρώστε το ΑΦΜ.");
            return;
        }
        if mode == MODE_GR_AADE {
            return self.aade(&afm);
        }
        let page = if mode == MODE_GR_VIES { P_GR } else { P_EU };
        let shown = if cc == "EL" { afm.clone() } else { format!("{cc}{afm}") };
        self.status.set_text(0, " Αναζήτηση στο VIES…");
        // ponytail: blocking κλήσεις στο UI thread (~1-2s), thread αν γίνει ενοχλητικό
        match vies(cc, &afm) {
            Some(json) => {
                self.fill(&json, &[page]);
                let name = find(&json, "onomasia").unwrap_or("");
                self.status.set_text(0, &format!(" {shown}: {name}  (VIES)"));
            }
            None => {
                self.fill(&Value::Null, &[page]);
                let fallback = self.fallback.check_state() == nwg::CheckBoxState::Checked;
                if mode == MODE_GR_VIES && fallback && self.has_creds() {
                    self.bars[MODE_GR_AADE].afm.set_text(&afm);
                    self.tabs.set_selected_tab(MODE_GR_AADE);
                    return self.aade(&afm);
                }
                self.status.set_text(0, " Δεν βρέθηκε στο VIES.");
                let msg = format!("Το ΑΦΜ {shown} δεν είναι έγκυρο στο VIES ή η υπηρεσία δεν είναι διαθέσιμη.");
                nwg::modal_error_message(&self.window, "VIES", &msg);
            }
        }
    }

    fn aade(&self, afm: &str) {
        if !self.has_creds() {
            let msg = "Συμπληρώστε κωδικό και συνθηματικό ΑΑΔΕ στις Παραμέτρους,\nή χρησιμοποιήστε την αναζήτηση «Ελλάδα – VIES».";
            nwg::modal_error_message(&self.window, "Στοιχεία", msg);
            return;
        }
        self.status.set_text(0, " Αναζήτηση στην ΑΑΔΕ…");
        match query(self.user.text().trim(), &self.pass.text(), afm).and_then(|x| parse(&x)) {
            Ok(json) => {
                self.fill(&json, &[P_BASIC, P_CALL]);
                self.acts.set_text(&acts_text(&json));
                match json.pointer("/error_rec/error_descr").and_then(Value::as_str) {
                    Some(d) => {
                        self.status.set_text(0, &format!(" Σφάλμα: {d}"));
                        nwg::modal_error_message(&self.window, "ΑΑΔΕ", d);
                    }
                    None => {
                        let name = find(&json, "onomasia").unwrap_or("");
                        self.status.set_text(0, &format!(" {afm}: {name}  (ΑΑΔΕ)"));
                    }
                }
                *self.last.borrow_mut() = Some(json);
                self.itabs.set_selected_tab(0);
            }
            Err(e) => {
                self.status.set_text(0, " Αποτυχία αναζήτησης.");
                nwg::modal_error_message(&self.window, "Σφάλμα", &e);
            }
        }
    }

    fn do_save(&self) {
        let res = creds().ok_or("keyring".to_string()).and_then(|e| {
            let v = self.fallback.check_state() == nwg::CheckBoxState::Checked;
            e.set_password(&format!("{}\n{}\n{}", self.user.text().trim(), self.pass.text(), v as u8))
                .map_err(|e| e.to_string())
        });
        match res {
            Ok(_) => nwg::modal_info_message(&self.window, "Αποθήκευση", "Οι παράμετροι αποθηκεύτηκαν."),
            Err(e) => nwg::modal_error_message(&self.window, "Αποθήκευση", &format!("Αποτυχία: {e}")),
        };
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
place_impl!(nwg::Label, nwg::TextInput, nwg::TextBox, nwg::Button, nwg::CheckBox, nwg::TabsContainer, nwg::ComboBox<String>, nwg::RichLabel);

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
        let bar = ui.bars.iter().position(|b| handle == b.search.handle || handle == b.afm.handle);
        match evt {
            Event::OnWindowClose if handle == ui.window.handle => nwg::stop_thread_dispatch(),
            Event::OnResize | Event::OnWindowMaximize if handle == ui.window.handle => ui.layout(),
            Event::OnMinMaxInfo => {
                if let EventData::OnMinMaxInfo(info) = &data {
                    info.set_min_size(MIN_W, MIN_H);
                }
            }
            Event::OnButtonClick | Event::OnKeyEnter if bar.is_some() => ui.do_search(bar.unwrap()),
            Event::OnButtonClick if handle == ui.save.handle => ui.do_save(),
            // checkbox ΦΠΑ μόνο για ανάγνωση: επαναφορά της τιμής της ΑΑΔΕ
            Event::OnButtonClick => {
                if ui.fields.iter().any(|f| matches!(&f.val, Val::Check(c) if handle == c.handle)) {
                    ui.fill(ui.last.borrow().as_ref().unwrap_or(&Value::Null), &[P_BASIC]);
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
        assert_eq!(split_address("ΑΙΟΛΟΥ 86        10559 - ΑΘΗΝΑ"), ["ΑΙΟΛΟΥ", "86", "10559", "ΑΘΗΝΑ"].map(String::from));
        assert_eq!(split_address("VIA RICHINI 6\n20122 MILANO MI"), ["VIA RICHINI", "6", "20122", "MILANO MI"].map(String::from));
        assert_eq!(clean_vat("DE", "de 123.456.789"), "123456789");
        assert_eq!(clean_vat("EL", "GR094019245"), "094019245");
        assert_eq!(clean_vat("EL", "094019245"), "094019245");
        assert_eq!(clean_vat("NL", "NL123456789B01"), "123456789B01");
        assert_eq!(split_address("ΚΑΠΟΙΑ ΟΔΟΣ"), ["ΚΑΠΟΙΑ ΟΔΟΣ", "", "", ""].map(String::from));
    }
}

use std::fs::{self, File};
use std::path::{PathBuf, Path};
use std::time::SystemTime;
use std::collections::hash_map::{HashMap, Entry};
use std::sync::{Arc, Mutex};

use serde_json::from_reader;

use rocket::Request;
use rocket::request::{FromRequest, State, Outcome};

use rocket_accept_language::AcceptLanguage;

pub type SharedLanguageCache = Arc<Mutex<LanguageCache>>;

#[inline]
pub fn new_shared_language_cache() -> SharedLanguageCache {
    Arc::new(Mutex::new(LanguageCache::default()))
}

#[derive(Debug, Clone, Default)]
pub struct LanguageCache {
    inner: HashMap<String, CachedLanguage>,
}

impl LanguageCache {
    pub fn get(&mut self, code: &str) -> Option<Language> {
        match self.inner.entry(code.to_owned()) {
            Entry::Occupied(mut oe) => {
                let file_last_modified = fs::metadata(path(code)?)
                    .map(|m| m.modified().expect("Unsupported platform"))
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                
                if oe.get().last_modified < file_last_modified {
                    if let Some(lang) = CachedLanguage::read(code) {
                        *oe.get_mut() = lang;
                        Some(oe.get().language.clone())
                    } else {
                        oe.remove();
                        None
                    }
                } else {
                    Some(oe.get().language.clone())
                }
            }
            Entry::Vacant(ve) => {
                CachedLanguage::read(code).map(|lang| {
                    ve.insert(lang).language.clone()
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
struct CachedLanguage {
    language: Language,
    last_modified: SystemTime,
}

impl CachedLanguage {
    fn read(code: &str) -> Option<Self> {
        let file = File::open(path(code)?).ok()?;
        eprintln!("Reading {}", code);
        Some(CachedLanguage {
            last_modified: file.metadata().ok()?.modified().expect("Unsupported platform"),
            language: from_reader(file).ok()?,
        })
    }
}

fn path(code: &str) -> Option<PathBuf> {
    let languages = Path::new("languages/");

    let mut path = languages.join(code);
    path.set_extension("json");
    if path.parent() != Some(languages) {
        None
    } else {
        Some(path)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LangIcon {
    code: String,
    name: String
}

pub fn langs(lc: &mut LanguageCache) -> Vec<LangIcon> {
    Path::new("languages/").read_dir().unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let code = path.file_stem()?.to_os_string().into_string().ok()?;

            let lang = lc.get(&code)?;
            Some(LangIcon {
                code,
                name: lang.display_name,
            })
        })
        .collect()
}

impl FromRequest<'_, '_> for Language {
    type Error = ();
    fn from_request(req: &Request) -> Outcome<Self, Self::Error> {
        let slc = req.guard::<State<SharedLanguageCache>>()?.inner().clone();
        let mut lc = slc.lock().unwrap();

        let cookies = req.cookies();
        let code = if let Some(cookie) = cookies.get("lang") {
            cookie.value()
        } else {
            for locale in AcceptLanguage::from_request(req).unwrap().accept_language {
                let code = locale.get_language();

                if let Some(lang) = lc.get(code) {
                    return Outcome::Success(lang);
                }
            }
            ""
        };
        Outcome::Success(
            lc.get(code).unwrap_or_else(|| lc.get("no").unwrap())
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Language {
    pub lang_code: String,
    pub display_name: String,
    pub cookie_accept: String,
    pub index_title: String,
    pub welcome: String,
    pub new_game: String,
    pub or_join: String,
    pub game_code: String,
    pub join: String,

    pub not_found_title: String,
    pub the_page_was_not_found: String,

    pub game_title: String,
    pub special_thanks: String,
    pub write_to_opponent_here: String,

    pub rules: Rules,
    pub game: Game,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rules {
    rules: String,
    intro: String,
    placing_pieces_header: String,
    placing_pieces: String,
    moving_pieces_header: String,
    moving_pieces: String,
    taking_pieces_header: String,
    taking_pieces: String,
    hirdmann_killed_header: String,
    hirdmann_killed: String,
    aatakar_killed_header: String,
    aatakar_killed: String,
    who_wins_header: String,
    who_wins: String,
    king_killed_header: String,
    king_killed: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Game {
    close_error: String,
    end: TrueFalse,
    code: String,
    host_success: String,
    join_fail: String,
    join_success: String,
    your_turn: String,
    opponents_turn: String,
    game_start: String,
    game_start2: TrueFalse,
    game_win: String,
    game_lose: String,
    team_aatak: String,
    team_hirdi: String,
    unknown: String
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TrueFalse {
    r#true: String,
    r#false: String,
}
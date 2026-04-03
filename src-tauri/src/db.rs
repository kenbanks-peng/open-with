use crate::{App, Extension, Group, GroupDetail};
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct Database {
    conn: Connection,
    common_app_cache: HashMap<i64, HashSet<i64>>,
}

impl Database {
    pub fn open_or_create() -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn,
            common_app_cache: HashMap::new(),
        };
        db.create_tables()?;
        db.seed_extension_categories()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        path.push("file-type-groups.db");
        path
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS apps (
                id   INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS extensions (
                ext            TEXT PRIMARY KEY,
                group_id       INTEGER REFERENCES groups(id) ON DELETE SET NULL,
                description    TEXT NOT NULL DEFAULT '',
                default_app_id INTEGER REFERENCES apps(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS ext_apps (
                ext    TEXT NOT NULL REFERENCES extensions(ext) ON DELETE CASCADE,
                app_id INTEGER NOT NULL REFERENCES apps(id) ON DELETE CASCADE,
                PRIMARY KEY (ext, app_id)
            );
            CREATE TABLE IF NOT EXISTS groups (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                assigned_app_id INTEGER REFERENCES apps(id) ON DELETE SET NULL
            );
            CREATE TABLE IF NOT EXISTS extension_categories (
                ext      TEXT PRIMARY KEY,
                category TEXT NOT NULL
            );",
        )
    }

    fn seed_extension_categories(&self) -> Result<()> {
        let count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM extension_categories", [], |r| {
                    r.get(0)
                })?;
        if count > 0 {
            return Ok(());
        }

        let categories: &[(&str, &[&str])] = &[
            (
                "audio",
                &[
                    "mp3", "wav", "wave", "aac", "flac", "ogg", "m4a", "m4b", "wma", "aiff", "aif",
                    "aifc", "alac", "opus", "ac3", "amr", "ape", "au", "caf", "dts", "m4r", "mka",
                    "mp2", "ra", "snd", "wv", "weba", "aob", "it", "m2a", "mlp", "mod", "mp1",
                    "mpa", "mpc", "oma", "rmi", "s3m", "spx", "voc", "vqf", "xa", "xm", "f4a",
                    "f4b", "aa3", "acm", "pcm", "vox", "tak", "tta", "oga", "aa", "aax", "adts",
                    "bwf", "cdda", "m4p", "mpga", "ram", "rmp", "sd2", "ul", "ulaw", "ulw",
                ],
            ),
            (
                "audio-project",
                &[
                    "aup", "aup3", "band", "logic", "logicx", "ptx", "ptf", "als", "flp", "rpp",
                    "sesx", "mid", "midi", "cst", "dpst", "gbproj", "gchdb", "logikcs", "mwand",
                    "patch", "pst", "sbk", "sdir", "kar", "smf",
                ],
            ),
            (
                "video",
                &[
                    "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp",
                    "vob", "ogv", "mts", "m2ts", "divx", "asf", "f4v", "rm", "rmvb", "anx", "axa",
                    "axv", "bik", "crf", "dvdmedia", "evo", "gxf", "m1v", "m2p", "m2t", "mt2s",
                    "nsv", "nuv", "ogx", "rec", "tod", "tp", "vro", "wtv", "mk3d", "f4p", "3g2",
                    "qt", "amv", "swf", "xvid", "yuv", "dv", "ogm", "dat", "mxf", "movpkg", "mcf",
                    "mks", "ps", "3gp2", "3gpp", "avchd", "bdm", "bdmv", "dav", "dif", "flc",
                    "fli", "m15", "m75", "mpe", "m2v", "mpg4", "qta", "sdv", "vfw", "wm",
                ],
            ),
            (
                "video-project",
                &[
                    "prproj",
                    "fcpx",
                    "fcpbundle",
                    "imovieproject",
                    "veg",
                    "kdenlive",
                    "mlt",
                    "aep",
                    "drp",
                    "imovielibrary",
                    "imovieevent",
                    "imovietrailer",
                    "imoviemobile",
                    "cboard",
                    "theater",
                    "tvlibrary",
                    "videoslibrary",
                    "screenstudio",
                    "screenstudiopreset",
                ],
            ),
            (
                "image",
                &[
                    "png",
                    "jpg",
                    "jpeg",
                    "gif",
                    "bmp",
                    "tiff",
                    "tif",
                    "webp",
                    "svg",
                    "ico",
                    "heic",
                    "heif",
                    "raw",
                    "cr2",
                    "nef",
                    "arw",
                    "dng",
                    "orf",
                    "rw2",
                    "pef",
                    "srw",
                    "cr3",
                    "icns",
                    "pbm",
                    "pgm",
                    "ppm",
                    "pnm",
                    "tga",
                    "exr",
                    "hdr",
                    "jp2",
                    "jpx",
                    "avif",
                    "jxl",
                    "3fr",
                    "ari",
                    "bay",
                    "cap",
                    "crw",
                    "dcr",
                    "dcs",
                    "drf",
                    "erf",
                    "fff",
                    "gpr",
                    "iiq",
                    "k25",
                    "kdc",
                    "lfp",
                    "mdc",
                    "mef",
                    "mos",
                    "mrw",
                    "nefx",
                    "nrw",
                    "pxn",
                    "r3d",
                    "raf",
                    "rwl",
                    "sr2",
                    "srf",
                    "x3f",
                    "eps",
                    "hdp",
                    "j2k",
                    "jpegxr",
                    "jxr",
                    "wdp",
                    "dxb",
                    "fh10",
                    "fh11",
                    "abr",
                    "aae",
                    "xmp",
                    "pvt",
                    "slm",
                    "aplibrary",
                    "migratedaplibrary",
                    "migratedphotolibrary",
                    "photolibrary",
                    "photosasset",
                    "photoslibrary",
                    "astc",
                    "avci",
                    "avcs",
                    "cel",
                    "cleanshot",
                    "dds",
                    "dib",
                    "fpx",
                    "heics",
                    "heifs",
                    "hif",
                    "j2c",
                    "jpe",
                    "jpf",
                    "ktx",
                    "mpo",
                    "pct",
                    "pic",
                    "pict",
                    "pfm",
                    "pntg",
                    "pvr",
                    "sgi",
                    "svgz",
                    "xbm",
                    "photo booth",
                ],
            ),
            (
                "image-project",
                &[
                    "psd",
                    "psb",
                    "xcf",
                    "ai",
                    "sketch",
                    "fig",
                    "afdesign",
                    "afphoto",
                    "procreate",
                    "af",
                    "afassets",
                    "afbrushes",
                    "afextensiondocument",
                    "affinity",
                    "affinity_designer",
                    "affinity_photo",
                    "afluts",
                    "afmacros",
                    "afpackage",
                    "afpub",
                    "afstudio",
                    "aftemplate",
                    "persona",
                    "psdt",
                ],
            ),
            (
                "image-project-backup",
                &[
                    "afdesign~backup~",
                    "afdesign~lock~",
                    "afdesign~tmp~",
                    "affinity~backup~",
                    "affinity~lock~",
                    "affinity~tmp~",
                    "afphoto~backup~",
                    "afphoto~lock~",
                    "afphoto~tmp~",
                    "afpub~backup~",
                    "afpub~lock~",
                    "afpub~tmp~",
                    "af~backup~",
                    "af~lock~",
                    "af~tmp~",
                ],
            ),
            (
                "document",
                &[
                    "pdf",
                    "doc",
                    "docx",
                    "odt",
                    "rtf",
                    "pages",
                    "wpd",
                    "wps",
                    "abw",
                    "gdoc",
                    "gdraw",
                    "gform",
                    "glink",
                    "gmaillayout",
                    "gmap",
                    "gnote",
                    "gprj",
                    "gscript",
                    "gsheet",
                    "gsite",
                    "gslides",
                    "gtable",
                    "gvid",
                    "gcse",
                    "gdrive",
                    "gjam",
                    "docm",
                    "dotm",
                    "dotx",
                    "dvi",
                    "lpdf",
                    "ofd",
                    "ott",
                    "pages-tef",
                    "pdfd",
                    "rtfd",
                    "sdw",
                    "stw",
                    "sxw",
                    "xdv",
                    "template",
                ],
            ),
            ("plain-text", &["txt", "text", "log", "nfo"]),
            (
                "spreadsheet",
                &[
                    "xls",
                    "xlsx",
                    "csv",
                    "tsv",
                    "ods",
                    "numbers",
                    "xlsb",
                    "xlsm",
                    "xlt",
                    "xltx",
                    "xlam",
                    "nmbtemplate",
                    "numbers-tef",
                    "xla",
                    "xltm",
                ],
            ),
            (
                "presentation",
                &[
                    "ppt", "pptx", "key", "odp", "key-tef", "kpdc", "kpf", "kth", "pot", "potm",
                    "potx", "pps", "ppsm", "ppsx", "pptm",
                ],
            ),
            (
                "email",
                &[
                    "eml",
                    "emltpl",
                    "mbox",
                    "mime",
                    "mme",
                    "olk15calattach",
                    "olk15category",
                    "olk15contact",
                    "olk15event",
                    "olk15group",
                    "olk15message",
                    "olk15note",
                    "olk15pref",
                    "olk15signature",
                    "olk15task",
                    "eragesoundset",
                    "rge",
                    "emlx",
                    "emlxpart",
                    "ewsmbox",
                    "imapmbox",
                ],
            ),
            (
                "contact",
                &[
                    "vcf",
                    "abcdp",
                    "ldif",
                    "ldi",
                    "abcdg",
                    "abbu",
                    "persistentcardmodel",
                    "vcard",
                ],
            ),
            (
                "productivity",
                &[
                    "ofocus",
                    "ofocus-archive",
                    "ofocus-backup",
                    "ofocus-lock",
                    "ofocus-perspective",
                    "omnifocusjs",
                    "omnifocusjsz",
                    "omnijs",
                    "omnijsz",
                ],
            ),
            (
                "source-code",
                &[
                    "rs",
                    "py",
                    "js",
                    "jsx",
                    "tsx",
                    "go",
                    "java",
                    "c",
                    "cpp",
                    "h",
                    "hpp",
                    "rb",
                    "php",
                    "swift",
                    "kt",
                    "scala",
                    "zig",
                    "hs",
                    "lua",
                    "pl",
                    "pm",
                    "r",
                    "m",
                    "mm",
                    "cs",
                    "fs",
                    "ex",
                    "exs",
                    "clj",
                    "erl",
                    "elm",
                    "v",
                    "vhdl",
                    "sv",
                    "ts",
                    "sql",
                    "css",
                    "scss",
                    "less",
                    "sass",
                    "hh",
                    "hxx",
                    "h++",
                    "cc",
                    "cxx",
                    "c++",
                    "csx",
                    "cls",
                    "jav",
                    "mjs",
                    "cjs",
                    "cljs",
                    "cljx",
                    "clojure",
                    "edn",
                    "fsi",
                    "fsx",
                    "fsscript",
                    "coffee",
                    "dart",
                    "groovy",
                    "vb",
                    "ml",
                    "mli",
                    "pl6",
                    "pm6",
                    "vue",
                    "erb",
                    "jade",
                    "pug",
                    "handlebars",
                    "hbs",
                    "ctp",
                    "dot",
                    "pyi",
                    "gemspec",
                    "asp",
                    "aspx",
                    "cshtml",
                    "jshtm",
                    "jsp",
                    "phtml",
                    "ascx",
                    "rkt",
                    "scm",
                    "gemfile",
                    "xul",
                    "shtml",
                    "csh",
                    "javascript",
                    "jscript",
                ],
            ),
            (
                "web",
                &[
                    "html", "htm", "xhtml", "mhtml", "jhtml", "xbl", "xht", "xhtm", "xsl", "xslt",
                    "mht", "shtm",
                ],
            ),
            (
                "web-meta",
                &[
                    "download",
                    "safariextz",
                    "url",
                    "webarchive",
                    "webbookmark",
                    "webhistory",
                    "webloc",
                    "crwebloc",
                ],
            ),
            (
                "config",
                &[
                    "json",
                    "yaml",
                    "yml",
                    "toml",
                    "xml",
                    "ini",
                    "conf",
                    "cfg",
                    "plist",
                    "properties",
                    "prefpane",
                    "sysprefex",
                    "saver",
                    "internetconnect",
                    "networkconnect",
                    "rayconfig",
                    "keyclu",
                    "format",
                    "rdf",
                    "pset",
                ],
            ),
            (
                "archive",
                &[
                    "zip",
                    "tar",
                    "gz",
                    "bz2",
                    "xz",
                    "7z",
                    "rar",
                    "tgz",
                    "lz",
                    "zst",
                    "lzma",
                    "iinaplgz",
                    "appdownload",
                ],
            ),
            ("installer", &["pkg", "deb", "rpm"]),
            (
                "disk-image",
                &["dmg", "iso", "img", "sparseimage", "sparsebundle"],
            ),
            ("vm-disk", &["vdi", "vmdk", "qcow2"]),
            (
                "font",
                &[
                    "ttf",
                    "otf",
                    "woff",
                    "woff2",
                    "eot",
                    "dfont",
                    "ttc",
                    "otc",
                    "sfont",
                    "cfr",
                    "affont",
                    "suit",
                    "typeface-backup",
                    "typeface-license",
                ],
            ),
            (
                "3d-model",
                &[
                    "obj", "fbx", "stl", "3ds", "dae", "gltf", "glb", "usdz", "usd", "usda",
                    "usdc", "ply", "abc", "mtlx",
                ],
            ),
            ("3d-project", &["blend"]),
            (
                "database",
                &[
                    "db",
                    "sqlite",
                    "sqlite3",
                    "mdb",
                    "accdb",
                    "kdbx",
                    "musicdb",
                    "musiclibrary",
                    "tvdb",
                    "ite",
                    "itl",
                    "itlp",
                ],
            ),
            (
                "ebook",
                &[
                    "epub", "mobi", "azw", "azw3", "fb2", "djvu", "cbr", "cbz", "help", "book",
                    "iba", "ibooks",
                ],
            ),
            (
                "markdown",
                &[
                    "md", "markdown", "mdoc", "mdown", "mdtext", "mdtxt", "mdwn", "mkd", "mkdn",
                ],
            ),
            ("markup", &["rst", "adoc", "tex", "latex", "org", "rt"]),
            (
                "script",
                &[
                    "sh",
                    "bash",
                    "zsh",
                    "fish",
                    "bat",
                    "cmd",
                    "ps1",
                    "psm1",
                    "bash_login",
                    "bash_logout",
                    "bash_profile",
                    "bashrc",
                    "profile",
                    "zlogin",
                    "zlogout",
                    "zprofile",
                    "zshenv",
                    "zshrc",
                    "psgi",
                    "t",
                    "pod",
                    "pp",
                    "workflow",
                    "action",
                    "caction",
                    "definition",
                    "command",
                    "tool",
                    "shortcut",
                    "wflow",
                ],
            ),
            (
                "binary",
                &[
                    "app", "exe", "dll", "so", "dylib", "wasm", "class", "jar", "o", "a",
                ],
            ),
            (
                "cad",
                &["dwg", "dxf", "step", "stp", "iges", "igs", "sat", "brep"],
            ),
            (
                "geospatial",
                &["shp", "geojson", "kml", "kmz", "gpx", "osm"],
            ),
            (
                "subtitle",
                &[
                    "aqt", "ass", "idx", "jss", "mpsub", "pjs", "smi", "srt", "ssa", "sub", "usf",
                    "utf",
                ],
            ),
            (
                "playlist",
                &[
                    "b4s", "gvp", "sdp", "xspf", "cdg", "xesc", "m3u8", "m3u", "pls", "cue", "asx",
                    "vlc",
                ],
            ),
            (
                "devconfig",
                &[
                    "gitattributes",
                    "gitconfig",
                    "gitignore",
                    "bowerrc",
                    "config",
                    "editorconfig",
                    "jscsrc",
                    "jshintrc",
                    "code-workspace",
                    "csproj",
                    "dtd",
                    "wxi",
                    "wxl",
                    "wxs",
                    "xaml",
                    "eyaml",
                    "eyml",
                    "cmake",
                    "makefile",
                    "mk",
                    "dockerfile",
                    "containerfile",
                    "gradle",
                    "diff",
                    "lock",
                    "ipynb",
                    "xcodeproj",
                    "xcworkspace",
                    "psd1",
                    "rhistory",
                    "rprofile",
                ],
            ),
            ("calendar", &["ics", "vcs", "icbu", "aplmodel", "vcal"]),
            (
                "color-swatch",
                &["color", "palette", "ase", "clr", "snapshot"],
            ),
            (
                "plugin",
                &[
                    "iinaplugin",
                    "mcpb",
                    "skill",
                    "dxt",
                    "crx",
                    "hvpl",
                    "qtpxcomposition",
                ],
            ),
            ("game-notation", &["game", "pgn"]),
            (
                "notes",
                &[
                    "fdf",
                    "notesairdropdocument",
                    "notesarchive",
                    "skim",
                    "stickiesappexport",
                ],
            ),
            ("backup", &["enex", "journalarchive", "structuredbackup"]),
            ("app-specific", &["daisydisk", "gpscan", "navtrace"]),
        ];

        let mut stmt = self.conn.prepare(
            "INSERT OR IGNORE INTO extension_categories (ext, category) VALUES (?1, ?2)",
        )?;
        for (category, exts) in categories {
            for ext in *exts {
                stmt.execute(params![ext, category])?;
            }
        }
        Ok(())
    }

    // --- App operations ---

    pub fn upsert_app(&self, name: &str, path: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO apps (name, path) VALUES (?1, ?2)
             ON CONFLICT(name) DO UPDATE SET path = excluded.path",
            params![name, path],
        )?;
        let id: i64 =
            self.conn
                .query_row("SELECT id FROM apps WHERE name = ?1", params![name], |r| {
                    r.get(0)
                })?;
        Ok(id)
    }

    pub fn upsert_ext_app(&self, ext: &str, app_id: i64, description: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO extensions (ext, description) VALUES (?1, ?2)",
            params![ext, description],
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO ext_apps (ext, app_id) VALUES (?1, ?2)",
            params![ext, app_id],
        )?;
        Ok(())
    }

    pub fn set_default_app(&self, ext: &str, app_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE extensions SET default_app_id = ?1 WHERE ext = ?2",
            params![app_id, ext],
        )?;
        Ok(())
    }

    pub fn remove_stale_ext_apps(&self, app_id: i64, current_exts: &[String]) -> Result<()> {
        if current_exts.is_empty() {
            self.conn
                .execute("DELETE FROM ext_apps WHERE app_id = ?1", params![app_id])?;
            return Ok(());
        }
        let placeholders: Vec<String> = current_exts
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 2))
            .collect();
        let sql = format!(
            "DELETE FROM ext_apps WHERE app_id = ?1 AND ext NOT IN ({})",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        param_values.push(Box::new(app_id));
        for ext in current_exts {
            param_values.push(Box::new(ext.clone()));
        }
        let refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        stmt.execute(refs.as_slice())?;
        Ok(())
    }

    // --- Query operations ---

    pub fn get_apps(&self, filter: Option<&str>) -> Result<Vec<App>> {
        let sql = "SELECT a.id, a.name, a.path, COUNT(ea.ext) as ext_count
                   FROM apps a
                   LEFT JOIN ext_apps ea ON ea.app_id = a.id
                   WHERE (?1 IS NULL OR a.name LIKE '%' || ?1 || '%')
                   GROUP BY a.id
                   ORDER BY a.name COLLATE NOCASE";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![filter], |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_groups(
        &self,
        app_filter_id: Option<i64>,
        assigned_only: bool,
    ) -> Result<Vec<Group>> {
        let mut groups = Vec::new();

        // Only include "Ungrouped" when not filtering by app
        if app_filter_id.is_none() {
            let ungrouped_count: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM extensions WHERE group_id IS NULL",
                [],
                |r| r.get(0),
            )?;
            groups.push(Group {
                id: -1,
                name: "Ungrouped".to_string(),
                assigned_app_id: None,
                assigned_app_name: None,
                ext_count: ungrouped_count,
            });
        }

        let sql = if app_filter_id.is_some() {
            if assigned_only {
                "SELECT g.id, g.name, g.assigned_app_id, a.name, COUNT(e.ext) as ext_count
                 FROM groups g
                 LEFT JOIN apps a ON a.id = g.assigned_app_id
                 LEFT JOIN extensions e ON e.group_id = g.id
                 WHERE g.assigned_app_id = ?1
                 GROUP BY g.id
                 HAVING ext_count > 0
                 ORDER BY g.name COLLATE NOCASE"
            } else {
                "SELECT g.id, g.name, g.assigned_app_id, a.name, COUNT(e.ext) as ext_count
                 FROM groups g
                 LEFT JOIN apps a ON a.id = g.assigned_app_id
                 LEFT JOIN extensions e ON e.group_id = g.id
                 WHERE EXISTS (
                    SELECT 1 FROM extensions e2
                    JOIN ext_apps ea ON ea.ext = e2.ext
                    WHERE e2.group_id = g.id AND ea.app_id = ?1
                 )
                 GROUP BY g.id
                 ORDER BY g.name COLLATE NOCASE"
            }
        } else {
            "SELECT g.id, g.name, g.assigned_app_id, a.name, COUNT(e.ext) as ext_count
             FROM groups g
             LEFT JOIN apps a ON a.id = g.assigned_app_id
             LEFT JOIN extensions e ON e.group_id = g.id
             WHERE ?1 IS NULL
             GROUP BY g.id
             ORDER BY g.name COLLATE NOCASE"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params![app_filter_id], |r| {
            Ok(Group {
                id: r.get(0)?,
                name: r.get(1)?,
                assigned_app_id: r.get(2)?,
                assigned_app_name: r.get(3)?,
                ext_count: r.get(4)?,
            })
        })?;
        for row in rows {
            groups.push(row?);
        }
        Ok(groups)
    }

    pub fn get_group_detail(&self, group_id: Option<i64>) -> Result<GroupDetail> {
        let (group, extensions) = match group_id {
            None | Some(-1) => {
                // Ungrouped
                let ungrouped_count: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM extensions WHERE group_id IS NULL",
                    [],
                    |r| r.get(0),
                )?;
                let group = Group {
                    id: -1,
                    name: "Ungrouped".to_string(),
                    assigned_app_id: None,
                    assigned_app_name: None,
                    ext_count: ungrouped_count,
                };
                let mut stmt = self.conn.prepare(
                    "SELECT ext, group_id, description FROM extensions WHERE group_id IS NULL ORDER BY ext",
                )?;
                let exts: Vec<Extension> = stmt
                    .query_map([], |r| {
                        Ok(Extension {
                            ext: r.get(0)?,
                            group_id: r.get(1)?,
                            description: r.get(2)?,
                        })
                    })?
                    .collect::<Result<_>>()?;
                (group, exts)
            }
            Some(gid) => {
                let group = self.conn.query_row(
                    "SELECT g.id, g.name, g.assigned_app_id, a.name,
                        (SELECT COUNT(*) FROM extensions WHERE group_id = g.id)
                 FROM groups g LEFT JOIN apps a ON a.id = g.assigned_app_id
                 WHERE g.id = ?1",
                    params![gid],
                    |r| {
                        Ok(Group {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            assigned_app_id: r.get(2)?,
                            assigned_app_name: r.get(3)?,
                            ext_count: r.get(4)?,
                        })
                    },
                )?;
                let mut stmt = self.conn.prepare(
                "SELECT ext, group_id, description FROM extensions WHERE group_id = ?1 ORDER BY ext",
            )?;
                let exts: Vec<Extension> = stmt
                    .query_map(params![gid], |r| {
                        Ok(Extension {
                            ext: r.get(0)?,
                            group_id: r.get(1)?,
                            description: r.get(2)?,
                        })
                    })?
                    .collect::<Result<_>>()?;
                (group, exts)
            }
        };

        let common_apps = if group.id == -1 {
            Vec::new()
        } else {
            self.compute_common_apps(group.id)?
        };

        Ok(GroupDetail {
            group,
            extensions,
            common_apps,
        })
    }

    // --- Constraint validation ---

    pub fn get_apps_for_extension(&self, ext: &str) -> Result<Vec<App>> {
        let ids = self.get_apps_for_ext(ext)?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT id, name, path, (SELECT COUNT(*) FROM ext_apps WHERE app_id = a.id) as ext_count
             FROM apps a WHERE id IN ({}) ORDER BY name COLLATE NOCASE",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let id_vec: Vec<i64> = ids.into_iter().collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = id_vec
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(refs.as_slice(), |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    fn get_apps_for_ext(&self, ext: &str) -> Result<HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT app_id FROM ext_apps WHERE ext = ?1")?;
        let rows = stmt.query_map(params![ext], |r| r.get::<_, i64>(0))?;
        let mut set = HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    fn compute_common_apps_set(&self, group_id: i64) -> Result<HashSet<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT ext FROM extensions WHERE group_id = ?1")?;
        let exts: Vec<String> = stmt
            .query_map(params![group_id], |r| r.get::<_, String>(0))?
            .collect::<Result<_>>()?;

        if exts.is_empty() {
            return Ok(HashSet::new());
        }

        let mut common: Option<HashSet<i64>> = None;
        for ext in &exts {
            let apps = self.get_apps_for_ext(ext)?;
            common = Some(match common {
                None => apps,
                Some(c) => c.intersection(&apps).copied().collect(),
            });
        }
        Ok(common.unwrap_or_default())
    }

    fn compute_common_apps(&self, group_id: i64) -> Result<Vec<App>> {
        let ids = self.compute_common_apps_set(group_id)?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT id, name, path, (SELECT COUNT(*) FROM ext_apps WHERE app_id = a.id) as ext_count
             FROM apps a WHERE id IN ({}) ORDER BY name COLLATE NOCASE",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let id_vec: Vec<i64> = ids.into_iter().collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = id_vec
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(refs.as_slice(), |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_common_apps_for_app(&self, app_id: i64) -> Result<Vec<App>> {
        // Get all extensions this app supports
        let mut stmt = self
            .conn
            .prepare("SELECT ext FROM ext_apps WHERE app_id = ?1")?;
        let exts: Vec<String> = stmt
            .query_map(params![app_id], |r| r.get::<_, String>(0))?
            .collect::<Result<_>>()?;

        if exts.is_empty() {
            return Ok(Vec::new());
        }

        // Find apps that support ALL of these extensions
        let mut common: Option<HashSet<i64>> = None;
        for ext in &exts {
            let apps = self.get_apps_for_ext(ext)?;
            common = Some(match common {
                None => apps,
                Some(c) => c.intersection(&apps).copied().collect(),
            });
        }

        let ids = common.unwrap_or_default();
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let sql = format!(
            "SELECT id, name, path, (SELECT COUNT(*) FROM ext_apps WHERE app_id = a.id) as ext_count
             FROM apps a WHERE id IN ({}) ORDER BY name COLLATE NOCASE",
            placeholders.join(",")
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let id_vec: Vec<i64> = ids.into_iter().collect();
        let refs: Vec<&dyn rusqlite::types::ToSql> = id_vec
            .iter()
            .map(|id| id as &dyn rusqlite::types::ToSql)
            .collect();
        let rows = stmt.query_map(refs.as_slice(), |r| {
            Ok(App {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                ext_count: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn validate_move(&mut self, exts: &[String], target_group_id: i64) -> Result<bool> {
        // Get common apps for target group (use cache)
        let group_apps = if let Some(cached) = self.common_app_cache.get(&target_group_id) {
            cached.clone()
        } else {
            let apps = self.compute_common_apps_set(target_group_id)?;
            self.common_app_cache.insert(target_group_id, apps.clone());
            apps
        };

        // For empty groups, any extension is valid
        let member_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM extensions WHERE group_id = ?1",
            params![target_group_id],
            |r| r.get(0),
        )?;

        if member_count == 0 {
            // Empty group: intersection of all dragged extensions' app sets must be non-empty
            let mut common: Option<HashSet<i64>> = None;
            for ext in exts {
                let apps = self.get_apps_for_ext(ext)?;
                common = Some(match common {
                    None => apps,
                    Some(c) => c.intersection(&apps).copied().collect(),
                });
            }
            return Ok(common.is_some_and(|c| !c.is_empty()));
        }

        // Non-empty group: intersect group's common apps with each extension's apps
        let mut result = group_apps;
        for ext in exts {
            let ext_apps = self.get_apps_for_ext(ext)?;
            result = result.intersection(&ext_apps).copied().collect();
            if result.is_empty() {
                return Ok(false);
            }
        }
        Ok(!result.is_empty())
    }

    // --- Mutation operations ---

    pub fn move_extensions(&mut self, exts: &[String], target_group_id: Option<i64>) -> Result<()> {
        let target = match target_group_id {
            Some(-1) | None => None,
            Some(id) => Some(id),
        };

        for ext in exts {
            self.conn.execute(
                "UPDATE extensions SET group_id = ?1 WHERE ext = ?2",
                params![target, ext],
            )?;
        }

        // Invalidate cache for affected groups
        self.common_app_cache.clear();

        // Check if assigned app is still valid for target group
        if let Some(gid) = target {
            self.revalidate_group_assignment(gid)?;
        }

        Ok(())
    }

    fn revalidate_group_assignment(&mut self, group_id: i64) -> Result<()> {
        let assigned: Option<i64> = self.conn.query_row(
            "SELECT assigned_app_id FROM groups WHERE id = ?1",
            params![group_id],
            |r| r.get(0),
        )?;

        if let Some(app_id) = assigned {
            let common = self.compute_common_apps_set(group_id)?;
            if !common.contains(&app_id) {
                self.conn.execute(
                    "UPDATE groups SET assigned_app_id = NULL WHERE id = ?1",
                    params![group_id],
                )?;
            }
        }
        Ok(())
    }

    pub fn create_group(&self, name: &str) -> Result<Group> {
        self.conn
            .execute("INSERT INTO groups (name) VALUES (?1)", params![name])?;
        let id = self.conn.last_insert_rowid();
        Ok(Group {
            id,
            name: name.to_string(),
            assigned_app_id: None,
            assigned_app_name: None,
            ext_count: 0,
        })
    }

    pub fn rename_group(&self, group_id: i64, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE groups SET name = ?1 WHERE id = ?2",
            params![name, group_id],
        )?;
        Ok(())
    }

    pub fn delete_group(&self, group_id: i64) -> Result<()> {
        // Move all extensions to ungrouped
        self.conn.execute(
            "UPDATE extensions SET group_id = NULL WHERE group_id = ?1",
            params![group_id],
        )?;
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
        Ok(())
    }

    pub fn assign_app_to_group(&self, group_id: i64, app_id: Option<i64>) -> Result<()> {
        self.conn.execute(
            "UPDATE groups SET assigned_app_id = ?1 WHERE id = ?2",
            params![app_id, group_id],
        )?;
        Ok(())
    }

    // --- Reconciliation ---

    pub fn reconcile_group_assignments(&self) -> Result<usize> {
        let mut stmt = self.conn.prepare(
            "SELECT g.id FROM groups g WHERE g.assigned_app_id IS NULL
             AND EXISTS (SELECT 1 FROM extensions WHERE group_id = g.id)",
        )?;
        let group_ids: Vec<i64> = stmt
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<Result<_>>()?;

        let mut fixed = 0;
        for gid in &group_ids {
            // Pick the app that is the system default for the most extensions in this group
            let best: Option<(i64, String)> = self
                .conn
                .query_row(
                    "SELECT e.default_app_id, a.name FROM extensions e
                 JOIN apps a ON a.id = e.default_app_id
                 WHERE e.group_id = ?1 AND e.default_app_id IS NOT NULL
                 GROUP BY e.default_app_id
                 ORDER BY COUNT(*) DESC, a.name ASC
                 LIMIT 1",
                    params![gid],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;

            if let Some((app_id, _)) = best {
                self.conn.execute(
                    "UPDATE groups SET assigned_app_id = ?1 WHERE id = ?2",
                    params![app_id, gid],
                )?;
                fixed += 1;
            }
        }
        Ok(fixed)
    }

    // --- Clustering ---

    pub fn auto_cluster_ungrouped(&mut self) -> Result<usize> {
        // Get ungrouped extensions with their apps
        let mut stmt = self.conn.prepare(
            "SELECT e.ext, ea.app_id FROM extensions e
             JOIN ext_apps ea ON ea.ext = e.ext
             WHERE e.group_id IS NULL",
        )?;
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_>>()?;

        if rows.is_empty() {
            return Ok(0);
        }

        // Build (category, app_id) -> [extensions] map
        let mut cluster_map: HashMap<(String, i64), Vec<String>> = HashMap::new();
        for (ext, app_id) in &rows {
            let cat = self
                .conn
                .query_row(
                    "SELECT category FROM extension_categories WHERE ext = ?1",
                    params![ext],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_else(|_| "other".to_string());
            cluster_map
                .entry((cat, *app_id))
                .or_default()
                .push(ext.clone());
        }

        // Deduplicate extensions within each cluster
        for exts in cluster_map.values_mut() {
            exts.sort();
            exts.dedup();
        }

        // Create a group for each (category, app) pair
        let mut groups_created = 0;
        let mut used_names: HashSet<String> = HashSet::new();
        {
            let mut stmt = self.conn.prepare("SELECT name FROM groups")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for row in rows {
                used_names.insert(row?.to_lowercase());
            }
        }

        for ((cat, app_id), exts) in &cluster_map {
            if exts.is_empty() {
                continue;
            }

            let group_name = deduplicate_name(cat, &used_names);
            used_names.insert(group_name.to_lowercase());

            self.conn.execute(
                "INSERT INTO groups (name, assigned_app_id) VALUES (?1, ?2)",
                params![group_name, app_id],
            )?;
            let group_id = self.conn.last_insert_rowid();

            for ext in exts {
                self.conn.execute(
                    "UPDATE extensions SET group_id = ?1 WHERE ext = ?2",
                    params![group_id, ext],
                )?;
            }
            groups_created += 1;
        }

        self.common_app_cache.clear();
        Ok(groups_created)
    }

    pub fn breakout_group(&mut self, group_id: i64) -> Result<usize> {
        // Get extensions in this group
        let mut stmt = self
            .conn
            .prepare("SELECT ext FROM extensions WHERE group_id = ?1")?;
        let exts: Vec<String> = stmt
            .query_map(params![group_id], |r| r.get::<_, String>(0))?
            .collect::<Result<_>>()?;

        if exts.len() <= 1 {
            return Ok(0);
        }

        // Build signature map: sorted app IDs -> extensions
        let mut sig_map: HashMap<String, Vec<String>> = HashMap::new();
        for ext in &exts {
            let apps = self.get_apps_for_ext(ext)?;
            let mut app_ids: Vec<i64> = apps.into_iter().collect();
            app_ids.sort();
            let sig = app_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            sig_map.entry(sig).or_default().push(ext.clone());
        }

        // If all extensions share the same signature, nothing to break out
        if sig_map.len() <= 1 {
            return Ok(0);
        }

        // Get the original group name for sub-group naming
        let base_name: String = self.conn.query_row(
            "SELECT name FROM groups WHERE id = ?1",
            params![group_id],
            |r| r.get(0),
        )?;

        let mut used_names: HashSet<String> = HashSet::new();
        {
            let mut stmt = self.conn.prepare("SELECT name FROM groups")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            for row in rows {
                used_names.insert(row?.to_lowercase());
            }
        }

        // Delete the original group (ungroups its extensions)
        self.conn.execute(
            "UPDATE extensions SET group_id = NULL WHERE group_id = ?1",
            params![group_id],
        )?;
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
        used_names.remove(&base_name.to_lowercase());

        // Create a sub-group for each app-signature cluster
        let mut groups_created = 0;
        for (sig, cluster_exts) in &sig_map {
            let app_ids: Vec<i64> = sig
                .split(',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();

            let group_name = deduplicate_name(&base_name, &used_names);
            used_names.insert(group_name.to_lowercase());

            // Pick the most specific app (fewest total extensions) to assign
            let assigned_app_id = if app_ids.is_empty() {
                None
            } else {
                let mut best: Option<(i64, i64)> = None;
                for &aid in &app_ids {
                    let count: i64 = self.conn.query_row(
                        "SELECT COUNT(*) FROM ext_apps WHERE app_id = ?1",
                        params![aid],
                        |r| r.get(0),
                    )?;
                    if best.is_none() || count < best.unwrap().1 {
                        best = Some((aid, count));
                    }
                }
                best.map(|(id, _)| id)
            };

            self.conn.execute(
                "INSERT INTO groups (name, assigned_app_id) VALUES (?1, ?2)",
                params![group_name, assigned_app_id],
            )?;
            let new_group_id = self.conn.last_insert_rowid();

            for ext in cluster_exts {
                self.conn.execute(
                    "UPDATE extensions SET group_id = ?1 WHERE ext = ?2",
                    params![new_group_id, ext],
                )?;
            }
            groups_created += 1;
        }

        self.common_app_cache.clear();
        Ok(groups_created)
    }

    pub fn get_summary(&self) -> Result<(i64, i64, i64)> {
        let app_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM apps", [], |r| r.get(0))?;
        let group_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM groups", [], |r| r.get(0))?;
        let ext_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM extensions", [], |r| r.get(0))?;
        Ok((app_count, group_count, ext_count))
    }

    pub fn all_extensions(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT ext FROM extensions")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn cleanup_orphan_extensions(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM extensions WHERE ext NOT IN (SELECT DISTINCT ext FROM ext_apps)",
            [],
        )?;
        Ok(())
    }
}

fn deduplicate_name(base: &str, used: &HashSet<String>) -> String {
    let lower = base.to_lowercase();
    if !used.contains(&lower) {
        return base.to_string();
    }
    let mut i = 2;
    loop {
        let candidate = format!("{} {}", base, i);
        if !used.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        i += 1;
    }
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("com.devbox.filetypegroups")
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".file-type-groups"))
    }
}

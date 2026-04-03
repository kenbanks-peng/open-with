use crate::db::Database;
use core_foundation::array::CFArray;
use core_foundation::base::{ItemRef, TCFType};
use core_foundation::error::CFErrorRef;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::CFURL;
use std::path::{Path, PathBuf};

type LSRolesMask = u32;
const K_LS_ROLES_ALL: LSRolesMask = 0xFFFFFFFF;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    static kUTTagClassFilenameExtension: CFStringRef;

    fn UTTypeCreatePreferredIdentifierForTag(
        tag_class: CFStringRef,
        tag: CFStringRef,
        conforming_to: CFStringRef,
    ) -> CFStringRef;

    fn UTTypeCopyAllTagsWithClass(
        uti: CFStringRef,
        tag_class: CFStringRef,
    ) -> *const core_foundation::array::__CFArray;

    fn LSCopyDefaultRoleHandlerForContentType(
        content_type: CFStringRef,
        role: LSRolesMask,
    ) -> CFStringRef;

    fn LSCopyAllRoleHandlersForContentType(
        content_type: CFStringRef,
        role: LSRolesMask,
    ) -> *const core_foundation::array::__CFArray;

    fn LSCopyApplicationURLsForBundleIdentifier(
        bundle_id: CFStringRef,
        out_error: *mut CFErrorRef,
    ) -> *const core_foundation::array::__CFArray;
}

/// Resolve a UTI to its associated file extensions.
fn extensions_for_uti(uti: &str) -> Vec<String> {
    let uti_cf = CFString::new(uti);
    let tag_class = unsafe { CFString::wrap_under_get_rule(kUTTagClassFilenameExtension) };
    let result = unsafe {
        UTTypeCopyAllTagsWithClass(uti_cf.as_concrete_TypeRef(), tag_class.as_concrete_TypeRef())
    };
    if result.is_null() {
        return Vec::new();
    }
    let arr: CFArray<CFString> = unsafe { CFArray::wrap_under_create_rule(result as *mut _) };
    arr.iter()
        .map(|s: ItemRef<'_, CFString>| s.to_string())
        .collect()
}

/// Query Launch Services for all apps that handle a given file extension.
/// Returns vec of (app_name, app_path) pairs.
fn ls_apps_for_extension(ext: &str) -> Vec<(String, String)> {
    let ext_cf = CFString::new(ext);
    let tag_class = unsafe { CFString::wrap_under_get_rule(kUTTagClassFilenameExtension) };

    // ext → UTI
    let uti_ref = unsafe {
        UTTypeCreatePreferredIdentifierForTag(
            tag_class.as_concrete_TypeRef(),
            ext_cf.as_concrete_TypeRef(),
            std::ptr::null(),
        )
    };
    if uti_ref.is_null() {
        return Vec::new();
    }
    let uti: CFString = unsafe { CFString::wrap_under_create_rule(uti_ref) };

    // UTI → bundle IDs
    let handlers_ref =
        unsafe { LSCopyAllRoleHandlersForContentType(uti.as_concrete_TypeRef(), K_LS_ROLES_ALL) };
    if handlers_ref.is_null() {
        return Vec::new();
    }
    let handlers: CFArray<CFString> =
        unsafe { CFArray::wrap_under_create_rule(handlers_ref as *mut _) };

    let mut results = Vec::new();
    for bundle_id in handlers.iter() {
        let bundle_id: ItemRef<'_, CFString> = bundle_id;
        // bundle ID → app URL
        let urls_ref = unsafe {
            LSCopyApplicationURLsForBundleIdentifier(
                bundle_id.as_concrete_TypeRef(),
                std::ptr::null_mut(),
            )
        };
        if urls_ref.is_null() {
            continue;
        }
        let urls: CFArray<CFURL> = unsafe { CFArray::wrap_under_create_rule(urls_ref as *mut _) };
        if let Some(url) = urls.iter().next() {
            let url: ItemRef<'_, CFURL> = url;
            if let Some(path) = url.to_path() {
                let path_str = path.to_string_lossy().to_string();
                // Derive name from .app bundle name
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                results.push((name, path_str));
            }
        }
    }
    results
}

/// Query Launch Services for the system default app for a given file extension.
/// Returns (app_name, app_path) if found.
fn ls_default_app_for_extension(ext: &str) -> Option<(String, String)> {
    let ext_cf = CFString::new(ext);
    let tag_class = unsafe { CFString::wrap_under_get_rule(kUTTagClassFilenameExtension) };

    let uti_ref = unsafe {
        UTTypeCreatePreferredIdentifierForTag(
            tag_class.as_concrete_TypeRef(),
            ext_cf.as_concrete_TypeRef(),
            std::ptr::null(),
        )
    };
    if uti_ref.is_null() {
        return None;
    }
    let uti: CFString = unsafe { CFString::wrap_under_create_rule(uti_ref) };

    let handler_ref =
        unsafe { LSCopyDefaultRoleHandlerForContentType(uti.as_concrete_TypeRef(), K_LS_ROLES_ALL) };
    if handler_ref.is_null() {
        return None;
    }
    let bundle_id: CFString = unsafe { CFString::wrap_under_create_rule(handler_ref) };

    let urls_ref = unsafe {
        LSCopyApplicationURLsForBundleIdentifier(
            bundle_id.as_concrete_TypeRef(),
            std::ptr::null_mut(),
        )
    };
    if urls_ref.is_null() {
        return None;
    }
    let urls: CFArray<CFURL> = unsafe { CFArray::wrap_under_create_rule(urls_ref as *mut _) };
    let url: ItemRef<'_, CFURL> = urls.iter().next()?;
    let path = url.to_path()?;
    let path_str = path.to_string_lossy().to_string();
    let name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    Some((name, path_str))
}

pub fn scan_and_populate(db: &Database) -> Result<String, Box<dyn std::error::Error>> {
    let dirs = scan_dirs();
    let mut app_count = 0u32;
    let mut ext_count = 0u32;

    // Phase 1: Scan app plists to discover extensions and direct associations
    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("app") {
                continue;
            }
            match process_app(db, &path) {
                Ok((_, exts)) => {
                    app_count += 1;
                    ext_count += exts as u32;
                }
                Err(e) => {
                    eprintln!("Skipping {}: {e}", path.display());
                }
            }
        }
    }

    // Phase 2: For each known extension, query Launch Services for additional handlers
    // and record the system default app
    let all_exts = db.all_extensions()?;
    let mut ls_count = 0u32;
    for ext in &all_exts {
        for (name, path) in ls_apps_for_extension(ext) {
            let app_id = db.upsert_app(&name, &path)?;
            db.upsert_ext_app(ext, app_id, "")?;
            ls_count += 1;
        }
        if let Some((name, path)) = ls_default_app_for_extension(ext) {
            let app_id = db.upsert_app(&name, &path)?;
            db.set_default_app(ext, app_id)?;
        }
    }

    db.cleanup_orphan_extensions()?;

    Ok(format!(
        "Scanned {app_count} apps, {ext_count} plist associations, {ls_count} Launch Services associations"
    ))
}

fn scan_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs
}

fn process_app(
    db: &Database,
    app_path: &Path,
) -> Result<(i64, usize), Box<dyn std::error::Error>> {
    let plist_path = app_path.join("Contents/Info.plist");
    if !plist_path.exists() {
        return Err(format!("No Info.plist at {}", plist_path.display()).into());
    }

    let plist_val = plist::Value::from_file(&plist_path)?;
    let dict = plist_val
        .as_dictionary()
        .ok_or("plist is not a dictionary")?;

    // Get app name
    let name = dict
        .get("CFBundleDisplayName")
        .or_else(|| dict.get("CFBundleName"))
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            app_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let app_path_str = app_path.to_string_lossy().to_string();
    let app_id = db.upsert_app(&name, &app_path_str)?;

    // Extract document types
    let mut extensions_found = Vec::new();
    if let Some(doc_types) = dict.get("CFBundleDocumentTypes").and_then(|v| v.as_array()) {
        for doc_type in doc_types {
            let doc_dict = match doc_type.as_dictionary() {
                Some(d) => d,
                None => continue,
            };

            let description = doc_dict
                .get("CFBundleTypeName")
                .and_then(|v| v.as_string())
                .unwrap_or("");

            // Collect extensions from both legacy and modern keys
            let mut doc_exts = Vec::new();

            // Legacy: CFBundleTypeExtensions (e.g. Cursor)
            if let Some(exts) = doc_dict
                .get("CFBundleTypeExtensions")
                .and_then(|v| v.as_array())
            {
                for ext_val in exts {
                    if let Some(ext) = ext_val.as_string() {
                        doc_exts.push(ext.to_lowercase());
                    }
                }
            }

            // Modern: LSItemContentTypes → resolve UTIs to extensions (e.g. Chrome, TextEdit)
            if let Some(utis) = doc_dict
                .get("LSItemContentTypes")
                .and_then(|v| v.as_array())
            {
                for uti_val in utis {
                    if let Some(uti) = uti_val.as_string() {
                        for ext in extensions_for_uti(uti) {
                            doc_exts.push(ext.to_lowercase());
                        }
                    }
                }
            }

            for ext_lower in doc_exts {
                if ext_lower == "*" || ext_lower.is_empty() {
                    continue;
                }
                db.upsert_ext_app(&ext_lower, app_id, description)?;
                extensions_found.push(ext_lower);
            }
        }
    }

    db.remove_stale_ext_apps(app_id, &extensions_found)?;

    Ok((app_id, extensions_found.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uti_resolves_plain_text() {
        let exts = extensions_for_uti("public.plain-text");
        assert!(
            exts.contains(&"txt".to_string()),
            "expected 'txt' in {exts:?}"
        );
    }

    #[test]
    fn uti_resolves_html() {
        let exts = extensions_for_uti("public.html");
        assert!(
            exts.contains(&"html".to_string()),
            "expected 'html' in {exts:?}"
        );
    }

    #[test]
    fn uti_resolves_yaml() {
        let exts = extensions_for_uti("public.yaml");
        println!("public.yaml -> {exts:?}");
        assert!(
            exts.contains(&"yaml".to_string()),
            "expected 'yaml' in {exts:?}"
        );
    }

    #[test]
    fn uti_resolves_json() {
        let exts = extensions_for_uti("public.json");
        println!("public.json -> {exts:?}");
        assert!(
            exts.contains(&"json".to_string()),
            "expected 'json' in {exts:?}"
        );
    }

    #[test]
    fn unknown_uti_returns_empty() {
        let exts = extensions_for_uti("com.example.nonexistent-type-xyzzy");
        assert!(
            exts.is_empty(),
            "expected empty for unknown UTI, got {exts:?}"
        );
    }

    #[test]
    fn ls_finds_multiple_yaml_handlers() {
        let apps = ls_apps_for_extension("yaml");
        println!("LS handlers for yaml: {apps:?}");
        assert!(
            apps.len() > 1,
            "expected multiple yaml handlers, got {apps:?}"
        );
    }
}

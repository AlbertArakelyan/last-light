//! Notes on disk: `<data dir>/notes/<folder>/<note>.md`, exactly one folder
//! deep. Plain Markdown files, so notes stay readable without the app.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::storage;

const EXTENSION: &str = "md";
/// Created on first run so there's somewhere to put the first note. It's an
/// ordinary folder otherwise: deleting it is fine and it won't come back.
pub const DEFAULT_FOLDER: &str = "general";
const MAX_NAME_CHARS: usize = 100;
/// Names Windows refuses for files and folders, whatever the extension.
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub struct NotesStore {
    root: PathBuf,
}

impl NotesStore {
    /// Opens the notes folder in the app data folder, creating it (with the
    /// default folder) on first run. Returns a warning to show if that fails.
    pub fn open() -> (Self, Option<String>) {
        let store = Self::at(storage::data_dir().join("notes"));
        let warning = store.init().err();
        (store, warning)
    }

    pub(super) fn at(root: PathBuf) -> Self {
        Self { root }
    }

    pub(super) fn init(&self) -> Result<(), String> {
        if self.root.exists() {
            return Ok(());
        }
        fs::create_dir_all(self.root.join(DEFAULT_FOLDER))
            .map_err(|e| format!("Can't create the notes folder {}: {e}", self.root.display()))
    }

    /// Folder names, sorted case-insensitively.
    pub fn folders(&self) -> Result<Vec<String>, String> {
        // file_name, not file_stem: a folder named "v1.2" has no extension.
        let mut folders = list_dir(&self.root, |path| {
            path.is_dir().then(|| path.file_name()).flatten()
        })?;
        sort_names(&mut folders);
        Ok(folders)
    }

    /// Note names (without `.md`) in `folder`, sorted case-insensitively.
    pub fn notes(&self, folder: &str) -> Result<Vec<String>, String> {
        let dir = self.folder_path(folder)?;
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut notes = list_dir(&dir, |path| {
            let is_note = path.is_file() && path.extension().is_some_and(|ext| ext == EXTENSION);
            is_note.then(|| path.file_stem()).flatten()
        })?;
        sort_names(&mut notes);
        Ok(notes)
    }

    /// Creates a folder and returns its cleaned-up name.
    pub fn create_folder(&self, name: &str) -> Result<String, String> {
        let name = validate_name(name)?;
        let path = self.folder_path(&name)?;
        if path.exists() {
            return Err(format!("A folder named \"{name}\" already exists."));
        }
        fs::create_dir_all(&path).map_err(|e| format!("Can't create the folder: {e}"))?;
        Ok(name)
    }

    /// Renames a folder and returns its new name. Never merges into an existing folder.
    pub fn rename_folder(&self, old: &str, new: &str) -> Result<String, String> {
        let new = validate_name(new)?;
        let old_path = self.folder_path(old)?;
        if !old_path.is_dir() {
            return Err(format!("The folder \"{old}\" no longer exists."));
        }
        rename(&old_path, &self.folder_path(&new)?, "folder", &new)?;
        Ok(new)
    }

    /// Deletes a folder and every note in it.
    pub fn delete_folder(&self, name: &str) -> Result<(), String> {
        let path = self.folder_path(name)?;
        // is_dir rather than exists, so this can never be aimed at a stray file.
        if !path.is_dir() {
            return Err(format!("The folder \"{name}\" no longer exists."));
        }
        fs::remove_dir_all(&path).map_err(|e| format!("Can't delete the folder: {e}"))
    }

    /// Creates an empty note and returns its cleaned-up name.
    pub fn create_note(&self, folder: &str, name: &str) -> Result<String, String> {
        let name = validate_name(name)?;
        let path = self.note_path(folder, &name)?;
        if path.exists() {
            return Err(format!(
                "A note named \"{name}\" already exists in {folder}."
            ));
        }
        fs::create_dir_all(self.folder_path(folder)?)
            .and_then(|()| fs::write(&path, ""))
            .map_err(|e| format!("Can't create the note: {e}"))?;
        Ok(name)
    }

    /// Renames a note within its folder and returns its new name.
    pub fn rename_note(&self, folder: &str, old: &str, new: &str) -> Result<String, String> {
        let new = validate_name(new)?;
        let old_path = self.note_path(folder, old)?;
        if !old_path.is_file() {
            return Err(format!("The note \"{old}\" no longer exists."));
        }
        rename(&old_path, &self.note_path(folder, &new)?, "note", &new)?;
        Ok(new)
    }

    pub fn delete_note(&self, folder: &str, name: &str) -> Result<(), String> {
        fs::remove_file(self.note_path(folder, name)?)
            .map_err(|e| format!("Can't delete the note: {e}"))
    }

    pub fn read_note(&self, folder: &str, name: &str) -> Result<String, String> {
        fs::read_to_string(self.note_path(folder, name)?)
            .map_err(|e| format!("Can't open the note: {e}"))
    }

    /// Saves a note, recreating its folder if it vanished. Writes to a temporary
    /// file and renames it over the note, so a crash or power loss mid-save
    /// leaves the previous version intact instead of a half-written file.
    pub fn write_note(&self, folder: &str, name: &str, content: &str) -> Result<(), String> {
        let path = self.note_path(folder, name)?;
        let temp = path.with_file_name(format!(".{name}.{EXTENSION}.tmp"));
        fs::create_dir_all(self.folder_path(folder)?)
            .and_then(|()| fs::write(&temp, content))
            .and_then(|()| fs::rename(&temp, &path))
            .map_err(|e| format!("Can't save the note: {e}"))
    }

    fn folder_path(&self, folder: &str) -> Result<PathBuf, String> {
        validate_name(folder)?;
        Ok(self.root.join(folder))
    }

    fn note_path(&self, folder: &str, name: &str) -> Result<PathBuf, String> {
        validate_name(name)?;
        Ok(self
            .folder_path(folder)?
            .join(format!("{name}.{EXTENSION}")))
    }
}

/// Trims a folder or note name and rejects anything that isn't a safe, portable
/// file name on Windows, macOS and Linux. Folder and note names share these rules.
fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The name can't be empty.".to_owned());
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(format!(
            "The name can be at most {MAX_NAME_CHARS} characters."
        ));
    }
    if name.starts_with('.') {
        return Err("The name can't start with a dot.".to_owned());
    }
    // Windows silently drops trailing dots, so "notes." would become "notes".
    if name.ends_with('.') {
        return Err("The name can't end with a dot.".to_owned());
    }
    if let Some(c) = name
        .chars()
        .find(|c| c.is_control() || r#"<>:"/\|?*"#.contains(*c))
    {
        return Err(format!("The name can't contain \"{c}\"."));
    }
    let stem = name.split('.').next().unwrap_or(name);
    if RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(stem))
    {
        return Err(format!("\"{name}\" is a reserved name on Windows."));
    }
    Ok(name.to_owned())
}

fn rename(old: &Path, new: &Path, kind: &str, new_name: &str) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    // On case-insensitive file systems a case-only rename ("todo" to "Todo")
    // finds the old file at the new path; that isn't a collision.
    let case_only = old.to_string_lossy().to_lowercase() == new.to_string_lossy().to_lowercase();
    if new.exists() && !case_only {
        return Err(format!("A {kind} named \"{new_name}\" already exists."));
    }
    fs::rename(old, new).map_err(|e| format!("Can't rename the {kind}: {e}"))
}

/// The names `name_of` picks out of the entries in `dir`. Hidden (dot) entries
/// are skipped, which also hides in-progress `.tmp` saves.
fn list_dir(dir: &Path, name_of: impl Fn(&Path) -> Option<&OsStr>) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Can't read {}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|entry| name_of(&entry.path())?.to_str().map(str::to_owned))
        .filter(|name| !name.starts_with('.'))
        .collect())
}

/// read_dir order is OS-dependent; sort so lists don't reshuffle between launches.
fn sort_names(names: &mut [String]) {
    names.sort_by_cached_key(|name| name.to_lowercase());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, NotesStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = NotesStore::at(dir.path().join("notes"));
        store.init().unwrap();
        (dir, store)
    }

    #[test]
    fn first_run_creates_the_default_folder_once() {
        let (_dir, store) = store();
        assert_eq!(store.folders().unwrap(), [DEFAULT_FOLDER]);
        store.delete_folder(DEFAULT_FOLDER).unwrap();
        store.init().unwrap();
        assert!(
            store.folders().unwrap().is_empty(),
            "a deleted default folder must not come back"
        );
    }

    #[test]
    fn notes_round_trip_inside_their_folder() {
        let (dir, store) = store();
        store.create_folder("Water").unwrap();
        store.create_note("Water", "Boiling").unwrap();
        store
            .write_note("Water", "Boiling", "# Boil for 1 minute")
            .unwrap();
        assert_eq!(
            store.read_note("Water", "Boiling").unwrap(),
            "# Boil for 1 minute"
        );
        assert!(dir.path().join("notes/Water/Boiling.md").is_file());
        assert_eq!(store.notes("Water").unwrap(), ["Boiling"]);
    }

    #[test]
    fn lists_are_sorted_and_skip_non_notes() {
        let (dir, store) = store();
        for name in ["b", "A", "c"] {
            store.create_note(DEFAULT_FOLDER, name).unwrap();
        }
        fs::write(dir.path().join("notes/general/image.png"), "").unwrap();
        fs::write(dir.path().join("notes/general/.hidden.md"), "").unwrap();
        store.write_note(DEFAULT_FOLDER, "b", "saved").unwrap();
        assert_eq!(store.notes(DEFAULT_FOLDER).unwrap(), ["A", "b", "c"]);

        store.create_folder("v1.2").unwrap();
        fs::create_dir(dir.path().join("notes/.git")).unwrap();
        assert_eq!(store.folders().unwrap(), [DEFAULT_FOLDER, "v1.2"]);
    }

    #[test]
    fn names_are_validated() {
        let (_dir, store) = store();
        for bad in [
            "",
            "   ",
            "a/b",
            "a\\b",
            "..",
            ".hidden",
            "trailing.",
            "what?",
            "con",
            "LPT1.txt",
        ] {
            assert!(
                store.create_folder(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
        assert_eq!(store.create_folder("  Trip plans  ").unwrap(), "Trip plans");
    }

    #[test]
    fn creating_or_renaming_onto_an_existing_name_fails() {
        let (_dir, store) = store();
        store.create_folder("Food").unwrap();
        assert!(store.create_folder("Food").is_err());
        assert!(store.rename_folder(DEFAULT_FOLDER, "Food").is_err());
        store.create_note("Food", "Rice").unwrap();
        store.create_note("Food", "Beans").unwrap();
        assert!(store.rename_note("Food", "Rice", "Beans").is_err());
    }

    #[test]
    fn rename_moves_content_and_allows_case_only_changes() {
        let (_dir, store) = store();
        store.create_note(DEFAULT_FOLDER, "todo").unwrap();
        store
            .write_note(DEFAULT_FOLDER, "todo", "- [ ] water")
            .unwrap();
        assert_eq!(
            store.rename_note(DEFAULT_FOLDER, "todo", "Todo").unwrap(),
            "Todo"
        );
        assert_eq!(
            store.read_note(DEFAULT_FOLDER, "Todo").unwrap(),
            "- [ ] water"
        );
        assert_eq!(
            store.rename_folder(DEFAULT_FOLDER, "Inbox").unwrap(),
            "Inbox"
        );
        assert_eq!(store.notes("Inbox").unwrap(), ["Todo"]);
    }

    #[test]
    fn deleting_a_folder_deletes_its_notes() {
        let (dir, store) = store();
        store.create_note(DEFAULT_FOLDER, "x").unwrap();
        store.delete_folder(DEFAULT_FOLDER).unwrap();
        assert!(!dir.path().join("notes/general").exists());
        assert!(store.notes(DEFAULT_FOLDER).unwrap().is_empty());
    }
}

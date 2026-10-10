//! Mirror the local workspace hierarchy without adding assets to the project library.
use super::*;
use std::collections::HashSet;

pub const KINDS: [&str; 4] = ["Projects", "Assets", "Exports", "Plugins"];

#[derive(Clone)]
pub struct Destination {
    pub folders: Vec<String>,
}
impl Destination {
    pub fn display(&self) -> String {
        format!("{}/", self.folders.join("/"))
    }
    pub fn key(&self) -> String {
        // Hash the components separately so literal separators in Unix names cannot collide.
        let encoded = serde_json::to_vec(&self.folders).expect("folder strings serialize");
        format!("{:x}", Sha256::digest(encoded))
    }
    pub fn kind(&self) -> &str {
        &self.folders[1]
    }
}

#[derive(Default)]
pub struct Library {
    pub files: Vec<Project>,
    pub destinations: HashMap<PathBuf, Destination>,
    pub note: String,
}

fn nested_folders(path: &Path, base: &Path) -> Vec<String> {
    path.parent()
        .and_then(|parent| parent.strip_prefix(base).ok())
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|part| match part {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

fn workspace_destination(path: &Path) -> Option<(&'static AppInfo, Destination)> {
    for base in path.parent()?.ancestors() {
        let name = base.file_name()?.to_str()?;
        let Some(app) = APPS.iter().find(|app| app.name.eq_ignore_ascii_case(name)) else {
            continue;
        };
        let relative = path.strip_prefix(base).ok()?;
        let first = relative.components().next()?.as_os_str();
        let kind = KINDS
            .iter()
            .find(|kind| first.to_string_lossy().eq_ignore_ascii_case(kind));
        let (kind, content_root) = match kind {
            Some(kind) if relative.components().count() > 1 => (*kind, base.join(first)),
            // Older project folders stored documents directly in the app folder.
            _ => ("Projects", base.to_path_buf()),
        };
        let mut folders = vec![app.name.into(), kind.into()];
        folders.extend(nested_folders(path, &content_root));
        return Some((app, Destination { folders }));
    }
    None
}

fn destination(project: &Project, roots: &[PathBuf]) -> (&'static AppInfo, Destination) {
    if let Some(found) = workspace_destination(&project.path) {
        return found;
    }
    let mut folders = vec![project.app.name.into(), "Projects".into()];
    // The most specific connected folder defines the relative path for external projects.
    if let Some(root) = roots
        .iter()
        .filter(|root| project.path.starts_with(root))
        .max_by_key(|root| root.components().count())
    {
        folders.extend(nested_folders(&project.path, root));
    }
    (project.app, Destination { folders })
}

pub fn scan(roots: &[PathBuf], projects: &[Project]) -> Library {
    let mut library = Library::default();
    let mut seen = HashSet::new();
    for project in projects {
        let (app, folder) = destination(project, roots);
        let mut project = project.clone();
        project.app = app;
        seen.insert(project.path.clone());
        library.destinations.insert(project.path.clone(), folder);
        library.files.push(project);
    }
    let mut unreadable = false;
    'roots: for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                !entry.file_type().is_symlink()
                    && !name.starts_with('.')
                    && (!entry.file_type().is_dir()
                        || (!entry.path().join(".artcraft-backups").is_file()
                            && !entry.path().join(".artcraft-nas.json").is_file()
                            && ![
                                "node_modules",
                                "target",
                                "appdata",
                                "windows",
                                "program files",
                            ]
                            .contains(&name.as_str())))
            })
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    unreadable = true;
                    continue;
                }
            };
            if !entry.file_type().is_file() || seen.contains(entry.path()) {
                continue;
            }
            let Some((app, folder)) = workspace_destination(entry.path()) else {
                continue;
            };
            if library.files.len() >= 10000 {
                library.note = "Showing the first 10,000 cloud files. Connect a more specific folder to include other files.".into();
                break 'roots;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => {
                    unreadable = true;
                    continue;
                }
            };
            let path = entry.path().to_path_buf();
            seen.insert(path.clone());
            let project = Project {
                revision: project_revision(&path),
                title: entry.file_name().to_string_lossy().into_owned(),
                modified: metadata.modified().ok().map(DateTime::<Local>::from),
                size_bytes: metadata.len(),
                path: path.clone(),
                app,
                preview: None,
                preview_note: "Preview unavailable for this file",
            };
            library.destinations.insert(path, folder);
            library.files.push(project);
        }
    }
    if unreadable {
        library.note.push_str(
            " Some local folders could not be read. Check their permissions and refresh.",
        );
    }
    library.files.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.path.cmp(&b.path))
    });
    library
}

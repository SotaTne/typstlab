use crate::models::CollectionError;
use crate::models::template::Template;
use std::path::{Path, PathBuf};
use typstlab_proto::{Collection, Entity};

pub struct TemplateScope {
    pub project_root: PathBuf,
    pub relative_path: PathBuf,
}

typstlab_proto::impl_entity! {
    TemplateScope {
        fn path(&self) -> PathBuf {
            self.project_root.join(&self.relative_path)
        }
    }
}

impl TemplateScope {
    pub fn new(project_root: PathBuf, relative_path: PathBuf) -> Self {
        Self {
            project_root,
            relative_path,
        }
    }
}

impl Collection<Template, CollectionError> for TemplateScope {
    fn list(&self) -> Result<Vec<Template>, CollectionError> {
        let root = self.path();
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut templates = Vec::new();
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.path().is_dir() {
                continue;
            }
            if let Some(id) = entry.file_name().to_str() {
                templates.push(Template {
                    id: id.to_string(),
                    path: entry.path(),
                });
            }
        }
        Ok(templates)
    }

    fn resolve(&self, input: &str) -> Result<Option<Template>, CollectionError> {
        let scope_root = self.path();
        let input_path = Path::new(input);

        // IDとしての解決
        let potential_id_path = scope_root.join(input);
        if potential_id_path.is_dir() {
            return Ok(Some(Template {
                id: input.to_string(),
                path: potential_id_path,
            }));
        }

        // パスとしての解決
        let has_absolute_or_rooted_component = matches!(
            input_path.components().next(),
            Some(std::path::Component::RootDir | std::path::Component::Prefix(_))
        );

        let abs_path = if has_absolute_or_rooted_component {
            input_path.to_path_buf()
        } else {
            self.project_root.join(input_path)
        };

        let full_path = match std::fs::canonicalize(&abs_path) {
            Ok(full_path) => full_path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CollectionError::Io(error)),
        };

        if full_path.is_dir() {
            let id = full_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Ok(Some(Template {
                id,
                path: full_path,
            }));
        }

        Ok(None)
    }
}

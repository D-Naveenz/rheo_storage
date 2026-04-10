#![allow(clippy::unused_unit)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::Mutex;

use intercom::{ComError, ComResult, com_class, com_interface, com_library};
use rheo_storage_lib::{DirectoryStorage, FileStorage, SearchScope};

com_library!(class FileObject, class DirectoryObject);

fn com_error() -> ComError {
    ComError::E_FAIL
}

#[com_class(Self)]
#[derive(Default)]
/// COM-visible wrapper for file operations in `rheo_storage_lib`.
struct FileObject {
    path: Mutex<Option<String>>,
}

#[com_interface]
impl FileObject {
    /// Bind this object to a file-system path.
    fn open(&self, path: String) -> ComResult<()> {
        FileStorage::new(&path).map_err(|_| com_error())?;
        *self.path.lock().map_err(|_| com_error())? = Some(path);
        Ok(())
    }

    /// Get the current absolute path.
    fn full_path(&self) -> ComResult<String> {
        Ok(self.current_path()?.to_string_lossy().into_owned())
    }

    /// Get the current file name.
    fn name(&self) -> ComResult<String> {
        Ok(self.current_file()?.name().unwrap_or_default().to_owned())
    }

    /// Check whether the bound file currently exists.
    fn exists(&self) -> ComResult<bool> {
        Ok(self.current_path()?.is_file())
    }

    /// Read the file as UTF-8 text.
    fn read_text(&self) -> ComResult<String> {
        self.current_file()?
            .read_to_string()
            .map_err(|_| com_error())
    }

    /// Write UTF-8 text to the file.
    fn write_text(&self, text: String) -> ComResult<String> {
        let updated = self
            .current_file()?
            .write_string(text)
            .map_err(|_| com_error())?;
        Ok(updated.path().display().to_string())
    }

    /// Copy the file to a destination path.
    fn copy_to(&self, destination: String) -> ComResult<String> {
        let copied = self
            .current_file()?
            .copy_to(destination)
            .map_err(|_| com_error())?;
        Ok(copied.path().display().to_string())
    }

    /// Move the file to a destination path.
    fn move_to(&self, destination: String) -> ComResult<String> {
        let moved = self
            .current_file()?
            .move_to(destination)
            .map_err(|_| com_error())?;
        Ok(moved.path().display().to_string())
    }

    /// Delete the bound file.
    fn delete(&self) -> ComResult<()> {
        self.current_file()?.delete().map_err(|_| com_error())
    }

    /// Return the detected MIME type when available.
    fn mime_type(&self) -> ComResult<String> {
        Ok(self
            .current_file()?
            .info_with_analysis()
            .map_err(|_| com_error())?
            .mime_type()
            .map_err(|_| com_error())?
            .unwrap_or_default()
            .to_owned())
    }

    /// Return a human-friendly file type name.
    fn type_name(&self) -> ComResult<String> {
        Ok(self
            .current_file()?
            .info_with_analysis()
            .map_err(|_| com_error())?
            .type_name())
    }
}

impl FileObject {
    fn current_path(&self) -> ComResult<std::path::PathBuf> {
        self.path
            .lock()
            .map_err(|_| com_error())?
            .clone()
            .map(std::path::PathBuf::from)
            .ok_or_else(com_error)
    }

    fn current_file(&self) -> ComResult<FileStorage> {
        FileStorage::from_existing(self.current_path()?).map_err(|_| com_error())
    }
}

#[com_class(Self)]
#[derive(Default)]
/// COM-visible wrapper for directory operations in `rheo_storage_lib`.
struct DirectoryObject {
    path: Mutex<Option<String>>,
}

#[com_interface]
impl DirectoryObject {
    /// Bind this object to a directory path.
    fn open(&self, path: String) -> ComResult<()> {
        DirectoryStorage::new(&path).map_err(|_| com_error())?;
        *self.path.lock().map_err(|_| com_error())? = Some(path);
        Ok(())
    }

    /// Get the current absolute path.
    fn full_path(&self) -> ComResult<String> {
        Ok(self.current_path()?.to_string_lossy().into_owned())
    }

    /// Get the current directory name.
    fn name(&self) -> ComResult<String> {
        Ok(self
            .current_directory()?
            .name()
            .unwrap_or_default()
            .to_owned())
    }

    /// Check whether the bound directory currently exists.
    fn exists(&self) -> ComResult<bool> {
        Ok(self.current_path()?.is_dir())
    }

    /// Return the recursive file count.
    fn file_count(&self) -> ComResult<u64> {
        self.current_directory()?
            .info_with_summary()
            .map_err(|_| com_error())?
            .file_count()
            .map_err(|_| com_error())
    }

    /// Return the recursive subdirectory count.
    fn directory_count(&self) -> ComResult<u64> {
        self.current_directory()?
            .info_with_summary()
            .map_err(|_| com_error())?
            .directory_count()
            .map_err(|_| com_error())
    }

    /// List top-level child files as newline-separated paths.
    fn list_files(&self) -> ComResult<String> {
        let files = self
            .current_directory()?
            .files_matching("*", SearchScope::TopDirectoryOnly)
            .map_err(|_| com_error())?;
        Ok(files
            .iter()
            .map(|file| file.path().display().to_string())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    /// List top-level child directories as newline-separated paths.
    fn list_directories(&self) -> ComResult<String> {
        let directories = self
            .current_directory()?
            .directories_matching("*", SearchScope::TopDirectoryOnly)
            .map_err(|_| com_error())?;
        Ok(directories
            .iter()
            .map(|directory| directory.path().display().to_string())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    /// Copy the directory tree to a destination path.
    fn copy_to(&self, destination: String) -> ComResult<String> {
        let copied = self
            .current_directory()?
            .copy_to(destination)
            .map_err(|_| com_error())?;
        Ok(copied.path().display().to_string())
    }

    /// Move the directory tree to a destination path.
    fn move_to(&self, destination: String) -> ComResult<String> {
        let moved = self
            .current_directory()?
            .move_to(destination)
            .map_err(|_| com_error())?;
        Ok(moved.path().display().to_string())
    }

    /// Delete the bound directory recursively.
    fn delete(&self) -> ComResult<()> {
        self.current_directory()?.delete().map_err(|_| com_error())
    }
}

impl DirectoryObject {
    fn current_path(&self) -> ComResult<std::path::PathBuf> {
        self.path
            .lock()
            .map_err(|_| com_error())?
            .clone()
            .map(std::path::PathBuf::from)
            .ok_or_else(com_error)
    }

    fn current_directory(&self) -> ComResult<DirectoryStorage> {
        DirectoryStorage::from_existing(self.current_path()?).map_err(|_| com_error())
    }
}

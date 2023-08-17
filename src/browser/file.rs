use crate::MediaType;

#[derive(Clone)]
pub struct File {
    inner: web_sys::File,
}

// hazardous, but as JS is single threaded (everything is accessed from UI thread), it is OK
// unsafe impl Sync for File {}
// #[allow(clippy::non_send_fields_in_send_ty)]
// unsafe impl Send for File {}

impl From<web_sys::File> for File {
    fn from(file: web_sys::File) -> Self {
        Self { inner: file }
    }
}

impl From<File> for web_sys::File {
    fn from(file: File) -> Self {
        file.inner
    }
}

impl File {
    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn media_type(&self) -> MediaType {
        self.inner.type_().into()
    }
}

#[derive(PartialEq, Eq)]
pub struct FileList {
    inner: web_sys::FileList,
}

impl From<web_sys::FileList> for FileList {
    fn from(list: web_sys::FileList) -> Self {
        Self { inner: list }
    }
}

impl From<FileList> for web_sys::FileList {
    fn from(file: FileList) -> Self {
        file.inner
    }
}

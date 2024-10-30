use smol_str::SmolStr;

use crate::MediaType;

#[derive(Clone)]
pub struct File {
    inner: web_sys::File,
}

// hazardous, but as JS is single threaded (everything is accessed from UI thread), it is OK
unsafe impl Sync for File {}
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for File {}

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
    pub fn name(&self) -> SmolStr {
        self.inner.name().into()
    }

    pub fn media_type(&self) -> MediaType {
        self.inner.type_().as_str().into()
    }
}

#[derive(PartialEq, Eq)]
pub struct FileList {
    inner: web_sys::FileList,
}

impl FileList {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.inner.length() as usize
    }

    pub fn iter(&self) -> FileListIterator {
        FileListIterator::new(self)
    }
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

pub struct FileListIterator<'a> {
    list: &'a FileList,
    index: usize,
}

impl<'a> FileListIterator<'a> {
    pub fn new(list: &'a FileList) -> Self {
        Self { list, index: 0 }
    }
}

impl<'a> Iterator for FileListIterator<'a> {
    type Item = File;

    fn next(&mut self) -> Option<Self::Item> {
        self.list.inner.get(self.index as u32).map(|file| {
            self.index += 1;
            file.into()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.list.len()))
    }
}

use std::{cmp::max, collections::VecDeque};

use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceSlice {
    pub from: u64,
    pub to: u64,
}

impl SourceSlice {
    pub fn size(&self) -> u64 {
        if self.from <= self.to {
            self.to - self.from + 1
        } else {
            0
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Content {
    slices: VecDeque<SourceSlice>,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ContentError {
    #[error("bad offset '{0}' (expected range 0..{1})")]
    BadOffset(u64, u64),
}

impl Content {
    pub fn new() -> Self {
        Self {
            slices: VecDeque::new(),
        }
    }

    pub fn slices(&self) -> Vec<SourceSlice> {
        self.slices.iter().map(|s| s.to_owned()).collect()
    }

    pub fn write_back(&mut self, src_offset: u64, size: u64) {
        if size > 0 {
            self.slices.push_back(SourceSlice {
                from: src_offset,
                to: src_offset + size - 1,
            });
        }
    }

    pub fn write(
        &mut self,
        src_offset: u64,
        write_offset: u64,
        size: u64,
    ) -> Result<(), ContentError> {
        if write_offset == self.size() {
            self.write_back(src_offset, size);
            return Ok(());
        }
        if write_offset > self.size() {
            return Err(ContentError::BadOffset(write_offset, self.size()));
        }
        let old_size = self.size();
        if size > 0 {
            let mut current_offset = 0;
            let mut write_at_index = 0;
            for i in 0..self.slices.len() {
                let slice = &mut self.slices[i];
                let next_offset = current_offset + slice.size();
                if current_offset == write_offset {
                    write_at_index = i;
                    break;
                } else if next_offset > write_offset {
                    let old_size = slice.size();
                    let fst_half_to = slice.from + (write_offset - current_offset - 1);
                    let snd_half_to = slice.to;
                    slice.to = fst_half_to;
                    self.slices.insert(
                        i + 1,
                        SourceSlice {
                            from: fst_half_to + 1,
                            to: snd_half_to,
                        },
                    );
                    let fst_half_size = self.slices[i].size();
                    let snd_half_size = self.slices[i + 1].size();
                    assert!(
                        old_size == fst_half_size + snd_half_size,
                        "old: {}, fst: {}, snd: {}",
                        old_size,
                        fst_half_size,
                        snd_half_size
                    );
                    write_at_index = i + 1;
                    break;
                }
                current_offset = next_offset;
            }
            self.slices.insert(
                write_at_index,
                SourceSlice {
                    from: src_offset,
                    to: src_offset + size - 1,
                },
            );
            let truncate_from_index = write_at_index + 1;
            let mut truncate_size = size;
            for i in truncate_from_index..self.slices.len() {
                let slice = &mut self.slices[i];
                let can_truncate = slice.size();
                if can_truncate > truncate_size {
                    slice.from += truncate_size;
                    break;
                }
                slice.to = 0;
                truncate_size -= can_truncate;
            }
            self.slices.retain(|s| s.size() > 0);
        }
        let new_size = self.size();
        let expected_size = max(write_offset + size, old_size);
        assert!(
            new_size == expected_size,
            "new_size = {}, expected_size = {}:\n{:?}",
            new_size,
            expected_size,
            self.slices
        );
        for s in self.slices.iter() {
            assert!(s.size() > 0, "{:?}", self.slices);
        }
        Ok(())
    }

    pub fn read(&self, offset: u64, size: u64) -> Result<Content, ContentError> {
        if offset > self.size() {
            return Err(ContentError::BadOffset(offset, self.size()));
        }
        let mut content = Content::new();
        let mut current_offset = 0;
        let mut read_size = 0;
        let mut reading = false;
        let mut end_of_file = true;
        for s in self.slices.iter() {
            if reading {
                if read_size >= size {
                    end_of_file = false;
                    break;
                }
                let mut slice_read_size = s.size();
                if read_size + slice_read_size > size {
                    slice_read_size = size - read_size;
                }
                content.write_back(s.from, slice_read_size);
                read_size += slice_read_size;
            } else {
                let next_offset = current_offset + s.size();
                if next_offset > offset {
                    reading = true;
                    let read_from = offset - current_offset;
                    let mut slice_read_size = s.size() - read_from;
                    if slice_read_size > size {
                        slice_read_size = size;
                    }
                    content.write_back(s.from + read_from, slice_read_size);
                    read_size += slice_read_size;
                    continue;
                }
                current_offset = next_offset;
            }
        }
        assert!(
            content.size() == read_size || end_of_file && content.size() <= read_size,
            "read: {}, want: {}, end: {}",
            content.size(),
            read_size,
            end_of_file,
        );
        Ok(content)
    }

    pub fn size(&self) -> u64 {
        self.slices.iter().fold(0, |acc: u64, s| acc + s.size())
    }
}

#[cfg(test)]
mod tests {
    use crate::abstract_fs::content::ContentError;

    use super::Content;

    #[test]
    fn test_read_empty() {
        let content = Content::new();
        assert_eq!(
            Content::new().slices(),
            content.read(0, 0).unwrap().slices()
        );
    }

    #[test]
    fn test_read_bad_offset() {
        let content = Content::new();
        assert_eq!(Err(ContentError::BadOffset(42, 0)), content.read(42, 0));
    }

    #[test]
    fn test_read_from_start() {
        let mut content = Content::new();
        content.write_back(42, 100);
        content.write_back(1, 2);
        content.write_back(13, 55);
        assert_eq!(content, content.read(0, 1024).unwrap());
    }

    #[test]
    fn test_read_from_middle() {
        let mut content = Content::new();
        content.write_back(42, 100);
        content.write_back(1, 2);
        content.write_back(13, 55);
        let mut expected = Content::new();
        expected.write_back(42 + 95, 5);
        expected.write_back(1, 2);
        expected.write_back(13, 10 - 5 - 2);
        assert_eq!(expected, content.read(95, 10).unwrap());
    }

    #[test]
    fn test_read_end() {
        let mut content = Content::new();
        content.write_back(42, 100);
        content.write_back(1, 2);
        assert_eq!(Content::new(), content.read(102, 1024).unwrap());
    }

    #[test]
    fn test_write_empty() {
        let mut content = Content::new();
        content.write(42, 0, 100).unwrap();
        let mut expected = Content::new();
        expected.write_back(42, 100);
        assert_eq!(expected, content);
    }

    #[test]
    fn test_write_bad_offset() {
        let mut content = Content::new();
        assert_eq!(Err(ContentError::BadOffset(42, 0)), content.write(0, 42, 0));
    }

    #[test]
    fn test_write_in_middle() {
        let mut content = Content::new();
        content.write_back(42, 100);
        content.write_back(1, 2);
        content.write_back(13, 55);
        content.write(77, 33, 10).unwrap();
        let mut expected = Content::new();
        expected.write_back(42, 33);
        expected.write_back(77, 10);
        expected.write_back(42 + 33 + 10, 100 - 33 - 10);
        expected.write_back(1, 2);
        expected.write_back(13, 55);
        assert_eq!(expected, content);
    }

    #[test]
    fn test_write_append() {
        let mut content = Content::new();
        content.write_back(42, 100);
        content.write(13, 100, 55).unwrap();
        let mut expected = Content::new();
        expected.write_back(42, 100);
        expected.write_back(13, 55);
        assert_eq!(expected, content);
    }

    #[test]
    fn test_write_single() {
        let mut content = Content::new();
        content.write_back(1000, 1);
        content.write(1000, 0, 1).unwrap();
        let mut expected = Content::new();
        expected.write_back(1000, 1);
        assert_eq!(expected, content)
    }
}

use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceSlice {
    pub from: u64,
    pub to: u64,
}

impl SourceSlice {
    pub fn size(&self) -> u64 {
        self.to - self.from + 1
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Content {
    slices: VecDeque<SourceSlice>,
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

    pub fn write(&mut self, src_offset: u64, offset: u64, size: u64) {
        let old_sise = self.size();
        if size > 0 {
            let mut truncate_size = size;
            for slice in self.slices.iter_mut() {
                let can_truncate = slice.size();
                if can_truncate > truncate_size {
                    slice.from += truncate_size;
                    break;
                }
                slice.from = slice.to;
                truncate_size -= can_truncate;
            }
            self.slices.retain(|s| s.from != s.to);
            self.slices.push_front(SourceSlice {
                from: src_offset,
                to: src_offset + size - 1,
            });
        }
        let new_size = self.size();
        if size < old_sise {
            assert!(
                new_size == old_sise,
                "new_size = {}, old_size = {}:\n{:?}",
                new_size,
                old_sise,
                self.slices
            )
        } else {
            assert!(
                new_size == size,
                "new_size = {}, size = {}:\n{:?}",
                new_size,
                size,
                self.slices
            )
        }
        for s in self.slices.iter() {
            assert!(
                s.from < s.to,
                "from = {}, to = {}:\n{:?}",
                s.from,
                s.to,
                self.slices
            );
        }
    }

    pub fn read(&self, offset: u64, size: u64) -> Content {
        let mut content = Content::new();
        let mut current_offset = 0;
        let mut read_size = 0;
        let mut reading = false;
        for s in self.slices.iter() {
            if reading {
                if read_size >= size {
                    break;
                }
                let slice_read_size = if s.size() > size { size } else { s.size() };
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
        assert!(content.size() == read_size);
        content
    }

    pub fn size(&self) -> u64 {
        self.slices.iter().fold(0, |acc: u64, s| acc + s.size())
    }
}

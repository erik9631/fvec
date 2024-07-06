use std::alloc::{alloc, Layout};
use std::intrinsics::copy_nonoverlapping;
use std::ops::{Index, IndexMut};
use std::slice::from_raw_parts_mut;
use crate::fvec::FVec;


pub struct AVec<'a, T, const CHUNK_COUNT: usize = 4> {
    chunks_raw: *mut FVec<T>,
    chunks: &'a mut [FVec<T>],
    current_chunk: usize,
    copies_behind: u8,
    largest_chunk: usize,
}
impl<'a, T, const CHUNK_COUNT: usize> AVec<'a, T, CHUNK_COUNT> {
    const CHUNKS: usize = CHUNK_COUNT;
    const MASK : usize = CHUNK_COUNT - 1;
    pub fn new(size: usize) -> AVec<'a, T, CHUNK_COUNT> {

        let layout = Layout::array::<FVec<T>>(CHUNK_COUNT).expect("Failed to create memory layout");
        let chunks_raw = unsafe {
            alloc(
                layout
            ) as *mut FVec<T>
        };

        let mut iter = chunks_raw;
        let end = unsafe { chunks_raw.add(CHUNK_COUNT)};

        unsafe {
            *iter = FVec::new(size);
            iter = iter.add(1)
        }

        while iter != end {
            unsafe {
                *iter = (*iter.offset(-1)).grow_from_last();
            }
            iter = unsafe {iter.add(1)};
        }
        let chunks = unsafe { from_raw_parts_mut(chunks_raw, CHUNK_COUNT)};

        AVec {
            chunks_raw,
            chunks,
            current_chunk: 0,
            copies_behind: 0,
            largest_chunk: 3
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.chunks[self.current_chunk].len
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.chunks[self.largest_chunk].capacity
    }

    fn rotate_right(index: usize, rotations: usize) -> usize {
        return (index + rotations) & Self::MASK;
    }

    fn rotate_left(index: usize, rotations: usize) -> usize {
        return (index.wrapping_sub(rotations)) & Self::MASK;
    }

    pub fn push(&mut self, val: T){
        let chunk = &mut self.chunks[self.current_chunk];
        chunk.push_raw(val);
        if chunk.len == chunk.capacity{
            self.copies_behind += 1;
            self.current_chunk = Self::rotate_right(self.current_chunk, 1);
            self.grow();
        }
    }

    fn grow(&mut self){
        let mut copies_behind = self.copies_behind;
        let copies_behind_original = copies_behind;
        let current_chunk = self.current_chunk;

        while copies_behind != 0 {
            let new_chunk = self.chunks[self.largest_chunk].grow_from_last();
            self.largest_chunk = Self::rotate_right(self.largest_chunk, 1);

            let replace_index = Self::rotate_left(current_chunk, copies_behind as usize);
            let replace_chunk = &self.chunks[replace_index];
            let current_chunk = &self.chunks[current_chunk];
            unsafe {copy_nonoverlapping(replace_chunk.data, current_chunk.data, replace_chunk.len)};

            self.chunks[replace_index].free();
            self.chunks[replace_index] = new_chunk;
            copies_behind -= 1;
        }
        self.copies_behind -= copies_behind_original;
    }
}

impl<'a, T, const CHUNK_COUNT: usize> Index<usize> for AVec<'a, T, CHUNK_COUNT> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.chunks[self.current_chunk].index(index)
    }
}

impl<'a, T, const CHUNK_COUNT: usize> IndexMut<usize> for AVec<'a, T, CHUNK_COUNT> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.chunks[self.current_chunk].index_mut(index)
    }
}
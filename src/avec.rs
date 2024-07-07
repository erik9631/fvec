use std::alloc::{alloc, Layout};
use std::intrinsics::copy_nonoverlapping;
use std::ops::{Index, IndexMut};
use std::slice::from_raw_parts_mut;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::JoinHandle;
use crate::fvec::FVec;


const STOP: usize = 0;
const GROW: usize = 1;

pub struct AVec<'a, T, const CHUNK_COUNT: usize = 4> {
    chunks_raw: *mut FVec<T>,
    chunks: &'a mut [FVec<T>],
    current_chunk: usize,
    largest_chunk: usize, // Should be owned by the grow thread. Make it a local thread variable
    tx: Sender<(usize, usize)>,
    grow_thread: JoinHandle<()>
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

        let (tx, grow_thread) = Self::start_grow_thread(chunks_raw);

        AVec {
            chunks_raw,
            chunks,
            current_chunk: 0,
            largest_chunk: 3,
            tx,
            grow_thread
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
            self.current_chunk = Self::rotate_right(self.current_chunk, 1);
            self.tx.send((GROW, self.current_chunk)).expect("Failed to send grow message");
        }
    }

    fn start_grow_thread(chunks_raw: *mut FVec<T>) -> (Sender<(usize, usize)>, JoinHandle<()>){
        let (tx, rx) = mpsc::channel::<(usize, usize)>();
        let chunks = chunks_raw;
        let handle = thread::spawn(move || {
            let mut replace_index = 0;
            let mut largest_index = CHUNK_COUNT - 1;
            let chunks = unsafe {from_raw_parts_mut(chunks, CHUNK_COUNT)}; // They will never overlap so it is fine
            while let message = rx.recv().expect("Grow thread hung up!") {
                if message.0 == STOP {
                    break;
                }
                let current_index = message.1;
                let new_chunk = chunks[largest_index].grow_from_last();

                let replace_chunk = &chunks[replace_index];
                let current_chunk = &chunks[current_index];

                unsafe {copy_nonoverlapping(replace_chunk.data, current_chunk.data, replace_chunk.len)};

                chunks[replace_index].free();
                chunks[replace_index] = new_chunk;
            }
        });

        (tx, handle)
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
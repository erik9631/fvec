use std::{alloc, thread};
use std::alloc::Layout;
use std::intrinsics::copy_nonoverlapping;
use std::mem::{size_of, swap};
use std::ops::Deref;
use std::sync::{Arc, atomic, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize};
use std::sync::atomic::Ordering::Relaxed;

fn default_alloc_strategy<T: 'static>(vec: &BVec<T>) -> bool{
    // 75% of the capacity is used
    if vec.capacity() == 0{
        return true;
    }
    return vec.len() as f64 / vec.capacity() as f64 > 0.75;
}

pub struct SharedData<T>{
    data: AtomicPtr<T>,
    tail: AtomicPtr<T>,
    is_growing: AtomicBool,
    capacity: AtomicUsize,
    grow_pow: AtomicUsize,
    push_mutex: Mutex<()>,
    pop_mutex: Mutex<()>,
    len: AtomicUsize,
    layout: Option<RwLock<Layout>>,
}
pub struct BVec<T>{
    shared_data: Arc<SharedData<T>>,
    alloc_strategy: fn(vec: &Self) -> bool,
    grow_thread: Option<thread::JoinHandle<()>>,
}

impl<T: 'static> BVec<T>{
    pub fn new() -> Self {
        BVec {
            shared_data: Arc::new(SharedData{
                data: AtomicPtr::new(std::ptr::null_mut()),
                tail: AtomicPtr::new(std::ptr::null_mut()),
                is_growing: AtomicBool::new(false),
                capacity: AtomicUsize::new(0),
                grow_pow: AtomicUsize::new(4),
                len: AtomicUsize::new(0),
                push_mutex: Mutex::new(()),
                pop_mutex: Mutex::new(()),
                layout: None,
            }),
            alloc_strategy: default_alloc_strategy,
            grow_thread: None,
        }
    }

    pub fn capacity (&self) -> usize{
        return self.shared_data.capacity.load(atomic::Ordering::Relaxed);
    }

    pub fn len(&self) -> usize{
        return self.shared_data.len.load(atomic::Ordering::Relaxed);
    }
    pub fn push(&mut self, item: T){
        if (self.alloc_strategy)(self){
            self.grow();
        }
        let result = self.shared_data.push_mutex.lock();
        if result.is_err(){
            panic!("Failed to lock push mutex {}", result.err().unwrap());
        }

        if self.len() >= self.capacity(){
            if let Some(grow_thread) = self.grow_thread.take() {
                grow_thread.join().expect("Failed to join grow thread");
            }
        }

        let tail = self.shared_data.tail.load(Relaxed);
        unsafe{ *tail = item;}
        self.shared_data.len.fetch_add(1, Relaxed);
    }

    fn grow(&mut self){
        if self.shared_data.is_growing.load(Relaxed) {
            return;
        }
        self.shared_data.is_growing.store(true, Relaxed);
        self.shared_data.grow_pow.fetch_add(1, Relaxed);
        let shared = self.shared_data.clone();

        self.grow_thread = Some(thread::spawn(move ||{
            let (new_data, new_layout) = alloc_new(shared.grow_pow.load(Relaxed));
            let new_size = new_layout.size() / size_of::<T>();
            shared.capacity.store(new_size, Relaxed);
            if shared.data.load(Relaxed).is_null(){
                shared.data.store(new_data, Relaxed);
                shared.tail.store(shared.data.load(Relaxed), Relaxed);
                shared.is_growing.store(false, Relaxed);
                return;
            }
            let local_data = shared.data.load(Relaxed);

            // Copy the old data to the new data on a new thread
            let size_before_copy = shared.len.load(Relaxed);
            // In case of popping we can't do partial copies...
            let _pop_mutex = shared.pop_mutex.lock().expect("Failed to lock pop mutex");


            unsafe{copy_nonoverlapping(local_data, new_data, size_before_copy)};
            // try copy the rest of the data
            let _result = shared.push_mutex.lock().expect("Failed to lock push mutex");
            let local_len = shared.len.load(Relaxed);

            let remainder = local_len - size_before_copy;
            let offset = size_before_copy as isize;
            unsafe {
                copy_nonoverlapping(local_data.offset(offset), new_data.offset(offset), remainder);
                shared.tail.store(new_data.offset(local_len as isize), Relaxed);
            }

            if let Some(prev_layout) = shared.layout.as_ref() {
                // Deallocation
                let read_layout = prev_layout.read().expect("Failed to read layout for deallocation");
                unsafe { alloc::dealloc(local_data as *mut u8, read_layout.clone()) };

                // Update layout
                let mut write_layout = prev_layout.write().expect("Failed to lock layout for writing");
                *write_layout = new_layout;
            }
            shared.is_growing.store(false, Relaxed);

        }));
    }
}

fn dealloc<T>(ptr: *mut T, layout: &RwLock<Layout>){
    let read_layout = layout.read().expect("Failed to read layout");
    unsafe{alloc::dealloc(ptr as *mut u8, read_layout.clone())};
}
fn alloc_new<T>(pow: usize) -> (*mut T, Layout) {
    let base: usize = 2;

    let layout_result = Layout::array::<T>(base.pow(pow as u32));
    if layout_result.is_err(){
        panic!("Failed to create memory layout {}", layout_result.err().unwrap());
    }
    let layout = layout_result.unwrap();
    let new_data = unsafe{alloc::alloc(layout) as *mut T};
    if new_data.is_null(){
        panic!("Failed to allocate memory");
    }
    return (new_data, layout);
}
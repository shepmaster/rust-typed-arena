#![feature(placement_in_syntax, placement_new_protocol)]
#![feature(test)]
#![feature(box_syntax, box_heap)]

//! The arena, a fast but limited type of allocator.
//!
//! Arenas are a type of allocator that destroy the objects within,
//! all at once, once the arena itself is destroyed.
//! They do not support deallocation of individual objects while the arena itself is still alive.
//! The benefit of an arena is very fast allocation; just a vector push.
//!
//! This is an equivalent of
//! [`arena::TypedArena`](http://doc.rust-lang.org/arena/struct.TypedArena.html)
//! distributed with rustc, but is available of Rust beta/stable.
//!
//! It is slightly less efficient, but simpler internally and uses much less unsafe code.
//! It is based on a `Vec<Vec<T>>` instead of raw pointers and manual drops.

// Potential optimizations:
// 1) add and stabilize a method for in-place reallocation of vecs.
// 2) add and stabilize placement new.
// 3) use an iterator. This may add far too much unsafe code.

use std::cell::RefCell;
use std::cmp;
use std::mem;

#[cfg(test)]
mod test;
#[cfg(test)]
mod bench;

// Initial size in bytes.
const INITIAL_SIZE: usize = 1024;
// Minimum capacity. Must be larger than 0.
const MIN_CAPACITY: usize = 1;

pub struct Arena<T> {
    chunks: RefCell<ChunkList<T>>,
}

struct ChunkList<T> {
    current: Vec<T>,
    rest: Vec<Vec<T>>,
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        let size = cmp::max(1, mem::size_of::<T>());
        Arena::with_capacity(INITIAL_SIZE / size)
    }

    pub fn with_capacity(n: usize) -> Arena<T> {
        let n = cmp::max(MIN_CAPACITY, n);
        Arena {
            chunks: RefCell::new(ChunkList {
                current: Vec::with_capacity(n),
                rest: vec![]
            }),
        }
    }

    pub fn alloc(&self, value: T) -> &mut T {
        // TODO: When placement syntax becomes stable, replace this method with
        // in self { value }

        let mut chunks = self.chunks.borrow_mut();

        // At this point, the current chunk must have free capacity.
        let next_item_index = chunks.current.len();
        chunks.current.push(value);
        let new_item_ref = {
            let new_item_ref = &mut chunks.current[next_item_index];

            // Extend the lifetime from that of `chunks_borrow` to that of `self`.
            // This is OK because we’re careful to never move items
            // by never pushing to inner `Vec`s beyond their initial capacity.
            // The returned reference is unique (`&mut`):
            // the `Arena` never gives away references to existing items.
            unsafe { mem::transmute::<&mut T, &mut T>(new_item_ref) }
        };

        if chunks.current.len() == chunks.current.capacity() {
            chunks.grow();
        }

        new_item_ref
    }

    pub fn into_vec(self) -> Vec<T> {
        let mut chunks = self.chunks.into_inner();
        // keep order of allocation in the resulting Vec
        let n = chunks.rest.iter().fold(chunks.current.len(), |a, v| a + v.len());
        let mut result = Vec::with_capacity(n);
        for mut vec in chunks.rest {
            result.append(&mut vec);
        }
        result.append(&mut chunks.current);
        result
    }
}

impl<T> ChunkList<T> {
    #[inline(never)]
    #[cold]
    fn grow(&mut self) {
        // Replace the current chunk with a newly allocated chunk.
        let new_capacity = self.current.capacity().checked_mul(2).unwrap();
        let chunk = mem::replace(&mut self.current, Vec::with_capacity(new_capacity));
        self.rest.push(chunk);
    }
}

use std::ops::{Placer, Place, InPlace};

impl<'a, T: 'a> Placer<T> for &'a Arena<T> {
    type Place = ArenaPlace<'a, T>;

    fn make_place(self) -> Self::Place {
        let mut chunks = self.chunks.borrow_mut();

        // At this point, the current chunk must have free capacity.
        // This precondition is maintained in the `finalize` method
        let next_item_index = chunks.current.len();

        let next_item_ptr = unsafe {
            // Move the pointer for one more space, and then get the
            // (uninitialized!) location for the next piece
            // TODO:: How does this handle panics after this point?
            chunks.current.set_len(next_item_index + 1);
            chunks.current.get_unchecked_mut(next_item_index)
        };

        ArenaPlace {
            arena: self,
            ptr: next_item_ptr,
        }
    }
}

pub struct ArenaPlace<'a, T: 'a> {
    arena: &'a Arena<T>,
    ptr: *mut T,
}

impl<'a, T: 'a> Place<T> for ArenaPlace<'a, T> {
    fn pointer(&mut self) -> *mut T {
        self.ptr
    }
}

impl<'a, T: 'a> InPlace<T> for ArenaPlace<'a, T> {
    type Owner = &'a mut T;

    unsafe fn finalize(self) -> Self::Owner {
        let mut chunks = self.arena.chunks.borrow_mut();

        // Maintain the precondition that there will always be space
        // when we try to allocate next.
        if chunks.current.len() == chunks.current.capacity() {
            chunks.grow();
        }

        let ptr = self.ptr;

        // Forgetting the reference to the arena is safe as something
        // else owns it. This prevents the `Drop` implementation from
        // running, which would otherwise roll back the allocation.
        mem::forget(self);

        // Extend the lifetime to that of `arena`.
        //
        // This is OK because we’re careful to never move items as we
        // never push to the inner `Vec`s beyond their initial
        // capacity.
        //
        // The returned reference is unique (`&mut`) because the
        // `Arena` never gives away references to existing items.
        mem::transmute(ptr)
    }
}

impl<'a, T: 'a> Drop for ArenaPlace<'a, T> {
    fn drop(&mut self) {
        // We are only dropped if the placement fails. That means we
        // need to roll back the allocation so that the destructor of
        // T is not run on uninitialized memory.
        let mut chunks = self.arena.chunks.borrow_mut();
        let len = chunks.current.len();
        unsafe { chunks.current.set_len(len - 1) };
    }
}

//! 単純なフリーリスト方式のヒープアロケータ(アドレス順 + 隣接ブロックの結合)。
//! シングルコア・割り込み無効で動かす前提なのでロックは持たない。

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

const ALIGN: usize = 16;

struct Node {
    size: usize,
    next: *mut Node,
}

pub struct FreeListAllocator {
    head: UnsafeCell<*mut Node>,
}

unsafe impl Sync for FreeListAllocator {}

impl FreeListAllocator {
    pub const fn new() -> Self {
        Self {
            head: UnsafeCell::new(null_mut()),
        }
    }

    /// ヒープ領域 [start, start+size) を登録する。起動時に一度だけ呼ぶ。
    pub unsafe fn init(&self, start: usize, size: usize) {
        let begin = (start + ALIGN - 1) & !(ALIGN - 1);
        let end = (start + size) & !(ALIGN - 1);
        if end <= begin + ALIGN {
            return;
        }
        let node = begin as *mut Node;
        (*node).size = end - begin;
        (*node).next = null_mut();
        *self.head.get() = node;
    }
}

fn block_size(layout: &Layout) -> usize {
    let size = if layout.size() < ALIGN { ALIGN } else { layout.size() };
    (size + ALIGN - 1) & !(ALIGN - 1)
}

unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > ALIGN {
            return null_mut();
        }
        let size = block_size(&layout);
        let head = self.head.get();
        let mut prev: *mut Node = null_mut();
        let mut cur = *head;

        while !cur.is_null() {
            if (*cur).size >= size {
                let remaining = (*cur).size - size;
                let next = if remaining >= ALIGN {
                    // 余りを新しい空きブロックとして残す
                    let rest = (cur as usize + size) as *mut Node;
                    (*rest).size = remaining;
                    (*rest).next = (*cur).next;
                    rest
                } else {
                    (*cur).next
                };
                if prev.is_null() {
                    *head = next;
                } else {
                    (*prev).next = next;
                }
                return cur as *mut u8;
            }
            prev = cur;
            cur = (*cur).next;
        }
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = block_size(&layout);
        let node = ptr as *mut Node;
        (*node).size = size;

        let head = self.head.get();
        let mut prev: *mut Node = null_mut();
        let mut cur = *head;
        while !cur.is_null() && (cur as usize) < (node as usize) {
            prev = cur;
            cur = (*cur).next;
        }
        (*node).next = cur;

        // 後ろの空きブロックと結合
        if !cur.is_null() && (node as usize) + (*node).size == cur as usize {
            (*node).size += (*cur).size;
            (*node).next = (*cur).next;
        }

        // 前の空きブロックと結合(またはリストに挿入)
        if prev.is_null() {
            *head = node;
        } else if (prev as usize) + (*prev).size == node as usize {
            (*prev).size += (*node).size;
            (*prev).next = (*node).next;
        } else {
            (*prev).next = node;
        }
    }
}

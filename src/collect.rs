use crate::mark::MarkWord;
use crate::ptr::DirectObjUnknown;
use crate::trace::HeapObjectLayout;

pub unsafe trait VisitHeap {
    type Layout: HeapObjectLayout;
    type EntryIter: IntoIterator<Item = DirectObjUnknown>;

    fn iter_entries(&self) -> Self::EntryIter;

    unsafe fn unmark_heap(&self) {
        for entry in self.iter_entries() {
            Self::Layout::mark(entry).unmark();
        }
    }
}

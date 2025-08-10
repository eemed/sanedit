use sanedit_utils::appendlist::{Appendlist, Reader, Writer};

pub(crate) use sanedit_utils::appendlist::AppendResult;
pub(crate) type AddBuffer = Appendlist<u8>;
pub(crate) type AddBufferReader = Reader<u8>;
pub(crate) type AddBufferWriter = Writer<u8>;

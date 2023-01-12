use super::eol::EOL;

pub(crate) fn detect_encoding(buf: &[u8]) -> &'static encoding_rs::Encoding {
    let mut encoding_detector = chardetng::EncodingDetector::new();
    encoding_detector.feed(buf, true);
    encoding_detector.guess(None, true)
}

pub(crate) fn detect_line_ending(buf: &[u8]) -> EOL {
    todo!()
}

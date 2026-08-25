//! HTTP `Range` header parsing (RFC 9110 §14).
//!
//! Range support is what makes a shared video seekable in a phone browser and
//! a half-finished 10 GB download resumable. Only single ranges are honoured:
//! multipart/byteranges buys nothing for these use cases, and RFC 9110 allows
//! a server to ignore a range request it does not want to satisfy.

/// An inclusive byte range that has been validated against a known file size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    /// Inclusive, as in the `Content-Range` header.
    pub end: u64,
}

impl ByteRange {
    pub fn len(self) -> u64 {
        self.end.saturating_sub(self.start) + 1
    }

    pub fn is_empty(self) -> bool {
        false
    }

    /// The `Content-Range` value for a 206 response.
    pub fn content_range(self, total: u64) -> String {
        format!("bytes {}-{}/{}", self.start, self.end, total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeRequest {
    /// No usable `Range` header: serve the whole file with 200.
    None,
    /// Serve exactly this slice with 206.
    Satisfiable(ByteRange),
    /// Reply 416 with `Content-Range: bytes */<size>`.
    Unsatisfiable,
}

/// Parse a `Range` header against a known file size.
///
/// Anything malformed degrades to [`RangeRequest::None`] (serve everything)
/// rather than an error, which is both what the RFC allows and what keeps odd
/// clients working. Only a syntactically valid range that cannot be satisfied
/// produces [`RangeRequest::Unsatisfiable`].
pub fn parse_range(header: &str, file_size: u64) -> RangeRequest {
    let Some(spec) = header.trim().strip_prefix("bytes=") else {
        return RangeRequest::None;
    };

    let parts: Vec<&str> = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    // Multiple ranges would need a multipart body; decline and send it all.
    if parts.len() != 1 {
        return RangeRequest::None;
    }
    let Some(part) = parts.first() else {
        return RangeRequest::None;
    };
    let Some((raw_start, raw_end)) = part.split_once('-') else {
        return RangeRequest::None;
    };
    let (raw_start, raw_end) = (raw_start.trim(), raw_end.trim());

    // `bytes=-N`: the final N bytes.
    if raw_start.is_empty() {
        let Ok(suffix) = raw_end.parse::<u64>() else {
            return RangeRequest::None;
        };
        if suffix == 0 || file_size == 0 {
            return RangeRequest::Unsatisfiable;
        }
        let start = file_size.saturating_sub(suffix);
        return RangeRequest::Satisfiable(ByteRange {
            start,
            end: file_size - 1,
        });
    }

    let Ok(start) = raw_start.parse::<u64>() else {
        return RangeRequest::None;
    };

    // An empty end means "to the end of the file".
    let end = if raw_end.is_empty() {
        file_size.saturating_sub(1)
    } else {
        match raw_end.parse::<u64>() {
            Ok(value) => value.min(file_size.saturating_sub(1)),
            Err(_) => return RangeRequest::None,
        }
    };

    if file_size == 0 || start >= file_size || start > end {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Satisfiable(ByteRange { start, end })
}

/// `Content-Range` value for a 416 response.
pub fn unsatisfied_content_range(file_size: u64) -> String {
    format!("bytes */{file_size}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn satisfiable(header: &str, size: u64) -> ByteRange {
        match parse_range(header, size) {
            RangeRequest::Satisfiable(range) => range,
            other => panic!("expected a satisfiable range for {header:?}, got {other:?}"),
        }
    }

    #[test]
    fn a_closed_range_is_parsed_inclusively() {
        let range = satisfiable("bytes=0-499", 10_000);
        assert_eq!(range, ByteRange { start: 0, end: 499 });
        assert_eq!(range.len(), 500);
        assert_eq!(range.content_range(10_000), "bytes 0-499/10000");
    }

    #[test]
    fn an_open_ended_range_runs_to_the_last_byte() {
        let range = satisfiable("bytes=1000000-", 5_000_000);
        assert_eq!(
            range,
            ByteRange {
                start: 1_000_000,
                end: 4_999_999
            }
        );
        assert_eq!(range.len(), 4_000_000);
    }

    #[test]
    fn a_suffix_range_returns_the_tail() {
        assert_eq!(
            satisfiable("bytes=-500", 10_000),
            ByteRange {
                start: 9_500,
                end: 9_999
            }
        );
    }

    #[test]
    fn a_suffix_longer_than_the_file_returns_the_whole_file() {
        assert_eq!(
            satisfiable("bytes=-50000", 100),
            ByteRange { start: 0, end: 99 }
        );
    }

    #[test]
    fn an_end_past_the_file_is_clamped() {
        assert_eq!(
            satisfiable("bytes=90-99999", 100),
            ByteRange { start: 90, end: 99 }
        );
    }

    #[test]
    fn a_single_byte_range_is_valid() {
        let range = satisfiable("bytes=0-0", 10);
        assert_eq!(range.len(), 1);
    }

    #[test]
    fn the_final_byte_can_be_requested() {
        assert_eq!(
            satisfiable("bytes=99-", 100),
            ByteRange { start: 99, end: 99 }
        );
    }

    #[test]
    fn whitespace_around_the_spec_is_tolerated() {
        assert_eq!(
            satisfiable("  bytes=0-9 ", 100),
            ByteRange { start: 0, end: 9 }
        );
        assert_eq!(
            satisfiable("bytes= 0 - 9 ", 100),
            ByteRange { start: 0, end: 9 }
        );
    }

    #[test]
    fn a_start_beyond_the_file_is_unsatisfiable() {
        assert_eq!(
            parse_range("bytes=100-200", 100),
            RangeRequest::Unsatisfiable
        );
        assert_eq!(parse_range("bytes=100-", 100), RangeRequest::Unsatisfiable);
        assert_eq!(unsatisfied_content_range(100), "bytes */100");
    }

    #[test]
    fn an_inverted_range_is_unsatisfiable() {
        assert_eq!(
            parse_range("bytes=500-100", 10_000),
            RangeRequest::Unsatisfiable
        );
    }

    #[test]
    fn an_empty_file_cannot_satisfy_any_range() {
        assert_eq!(parse_range("bytes=0-", 0), RangeRequest::Unsatisfiable);
        assert_eq!(parse_range("bytes=0-10", 0), RangeRequest::Unsatisfiable);
        assert_eq!(parse_range("bytes=-1", 0), RangeRequest::Unsatisfiable);
    }

    #[test]
    fn a_zero_length_suffix_is_unsatisfiable() {
        assert_eq!(parse_range("bytes=-0", 100), RangeRequest::Unsatisfiable);
    }

    #[test]
    fn malformed_headers_fall_back_to_the_whole_file() {
        for header in [
            "",
            "bytes",
            "bytes=",
            "items=0-10",
            "bytes=abc-def",
            "bytes=0-abc",
            "bytes=--5",
            "bytes=9999999999999999999999999-",
            "0-100",
        ] {
            assert_eq!(parse_range(header, 1000), RangeRequest::None, "{header:?}");
        }
    }

    #[test]
    fn multiple_ranges_are_declined_rather_than_mishandled() {
        assert_eq!(parse_range("bytes=0-99,200-299", 1000), RangeRequest::None);
        assert_eq!(
            parse_range("bytes=0-99, 200-299, 400-499", 1000),
            RangeRequest::None
        );
    }

    #[test]
    fn very_large_files_are_handled_without_overflow() {
        let huge = 50 * 1024 * 1024 * 1024_u64; // 50 GB
        let range = satisfiable("bytes=53687091200-", huge + 1);
        assert_eq!(range.start, 53_687_091_200);
        assert_eq!(range.end, huge);
        assert_eq!(range.len(), 1);

        let tail = satisfiable("bytes=-1048576", huge);
        assert_eq!(tail.len(), 1_048_576);
    }

    #[test]
    fn a_range_covering_the_entire_file_is_still_a_206() {
        let range = satisfiable("bytes=0-", 1000);
        assert_eq!(range, ByteRange { start: 0, end: 999 });
        assert_eq!(range.len(), 1000);
    }
}

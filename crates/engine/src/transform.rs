use flate2::read::GzDecoder;
use std::io::Read;
use unveil_contracts::{
    ErrorCode, Finding, IpcError, MAX_GZIP_OUTPUT, MAX_GZIP_RATIO,
};

use crate::detect::{decode_b64, decode_hex, decode_percent, encode_b64, encode_hex};
use crate::js::{fold_literal_concats, parse_string, pretty_print};

#[derive(Debug)]
pub struct AdapterOutput {
    pub bytes: Vec<u8>,
}

pub fn apply_adapter(
    input: &[u8],
    finding: &Finding,
    adapter_id: &str,
) -> Result<AdapterOutput, IpcError> {
    let start = finding.range.start as usize;
    let end = finding.range.end as usize;
    if start > end || end > input.len() {
        return Err(IpcError::new(ErrorCode::ParseFailed, "error.range"));
    }
    match adapter_id {
        "enc.base64" => decode_range(input, start, end, |s| decode_b64(s, false)),
        "enc.base64url" => decode_range(input, start, end, |s| decode_b64(s, true)),
        "enc.base16" => decode_range(input, start, end, decode_hex),
        "enc.percent" => decode_range(input, start, end, decode_percent),
        "enc.js-escape" => decode_js_string(input, start, end),
        "compression.gzip" => inflate_gzip(input),
        "layout.minify" => pretty(input),
        "data.literal-concat" => concat_fold(input),
        _ => Err(IpcError::new(
            ErrorCode::AdapterUnsupported,
            "error.adapter_unsupported",
        )),
    }
}

pub fn preview_adapter(
    input: &[u8],
    finding: &Finding,
    adapter_id: &str,
) -> Result<AdapterOutput, IpcError> {
    apply_adapter(input, finding, adapter_id)
}

pub fn reencode_range(
    original: &[u8],
    start: usize,
    end: usize,
    adapter_id: &str,
    decoded: &[u8],
) -> Result<bool, IpcError> {
    let src = std::str::from_utf8(&original[start..end])
        .map_err(|_| IpcError::new(ErrorCode::InvalidEncoding, "error.invalid_encoding"))?;
    let encoded = match adapter_id {
        "enc.base64" => encode_b64(decoded, false, src.contains('=')),
        "enc.base64url" => encode_b64(decoded, true, false),
        "enc.base16" => encode_hex(decoded),
        "enc.percent" => decoded.iter().map(|b| format!("%{b:02X}")).collect::<String>(),
        _ => return Ok(false),
    };
    if adapter_id == "enc.percent" {
        let a = src.replace("-%", "").to_ascii_uppercase();
        let b = encoded.to_ascii_uppercase();
        return Ok(normalize_percent(src) == normalize_percent(&encoded) || a.contains(&b[..b.len().min(8)]));
    }
    if adapter_id == "enc.base16" {
        return Ok(src.eq_ignore_ascii_case(&encoded));
    }
    Ok(src == encoded || src.trim_end_matches('=') == encoded.trim_end_matches('='))
}

fn normalize_percent(s: &str) -> String {
    s.to_ascii_uppercase()
}

fn decode_range(
    input: &[u8],
    start: usize,
    end: usize,
    f: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<AdapterOutput, IpcError> {
    let text = std::str::from_utf8(&input[start..end])
        .map_err(|_| IpcError::new(ErrorCode::InvalidEncoding, "error.invalid_encoding"))?;
    let bytes = f(text).ok_or_else(|| IpcError::new(ErrorCode::ParseFailed, "error.parse_failed"))?;
    Ok(AdapterOutput { bytes })
}

fn decode_js_string(input: &[u8], start: usize, end: usize) -> Result<AdapterOutput, IpcError> {
    parse_string(&input[start..end], 0)
        .map(|(_, value)| AdapterOutput {
            bytes: value.into_bytes(),
        })
        .ok_or_else(|| IpcError::new(ErrorCode::ParseFailed, "error.parse_failed"))
}

fn inflate_gzip(input: &[u8]) -> Result<AdapterOutput, IpcError> {
    let mut decoder = GzDecoder::new(input);
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match decoder.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                out.extend_from_slice(&buf[..n]);
                if out.len() as u64 > MAX_GZIP_OUTPUT {
                    return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
                }
                let ratio = if input.is_empty() {
                    0
                } else {
                    (out.len() as u64) / (input.len() as u64).max(1)
                };
                if ratio > MAX_GZIP_RATIO && out.len() as u64 > 64 * 1024 {
                    return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
                }
            }
            Err(_) => return Err(IpcError::new(ErrorCode::ParseFailed, "error.parse_failed")),
        }
    }
    if input.len() > 0 && (out.len() as u64) / (input.len() as u64).max(1) > MAX_GZIP_RATIO {
        return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
    }
    Ok(AdapterOutput { bytes: out })
}

fn pretty(input: &[u8]) -> Result<AdapterOutput, IpcError> {
    let text = std::str::from_utf8(input)
        .map_err(|_| IpcError::new(ErrorCode::InvalidEncoding, "error.invalid_encoding"))?;
    Ok(AdapterOutput {
        bytes: pretty_print(text).into_bytes(),
    })
}

fn concat_fold(input: &[u8]) -> Result<AdapterOutput, IpcError> {
    let text = std::str::from_utf8(input)
        .map_err(|_| IpcError::new(ErrorCode::InvalidEncoding, "error.invalid_encoding"))?;
    Ok(AdapterOutput {
        bytes: fold_literal_concats(text).into_bytes(),
    })
}

//! GGUF header sanity check and metadata extraction
//! (docs/architecture.md section 7, Decision 11).
//!
//! Reads only the file header and metadata key-value section -- never the
//! tensor data -- so the check is cheap even for multi-gigabyte files.
//! Turns "silent launch failure against a corrupt or renamed file" into a
//! clear ModelFormatInvalid error at selection time.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::errors::{AppError, ErrorCode};

const GGUF_MAGIC: u32 = 0x4655_4747; // "GGUF" little-endian
/// Metadata parsing safety bounds; a well-formed model stays far below these.
const MAX_KV_COUNT: u64 = 4096;
const MAX_STRING_LEN: u64 = 1 << 20;

#[derive(Debug, Clone)]
pub struct GgufInfo {
    pub version: u32,
    /// `<arch>.context_length` from metadata, if present.
    pub trained_context_length: Option<u64>,
    /// `general.name` from metadata, if present.
    pub model_name: Option<String>,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::ModelFormatInvalid, Some(detail.into()))
}

struct Reader<R: Read> {
    inner: R,
}

impl<R: Read> Reader<R> {
    fn u32(&mut self) -> Result<u32, AppError> {
        let mut b = [0u8; 4];
        self.inner
            .read_exact(&mut b)
            .map_err(|e| invalid(format!("truncated header: {e}")))?;
        Ok(u32::from_le_bytes(b))
    }

    fn u64(&mut self) -> Result<u64, AppError> {
        let mut b = [0u8; 8];
        self.inner
            .read_exact(&mut b)
            .map_err(|e| invalid(format!("truncated header: {e}")))?;
        Ok(u64::from_le_bytes(b))
    }

    fn string(&mut self) -> Result<String, AppError> {
        let len = self.u64()?;
        if len > MAX_STRING_LEN {
            return Err(invalid(format!("metadata string too long: {len}")));
        }
        let mut buf = vec![0u8; len as usize];
        self.inner
            .read_exact(&mut buf)
            .map_err(|e| invalid(format!("truncated metadata string: {e}")))?;
        String::from_utf8(buf).map_err(|_| invalid("metadata string is not UTF-8"))
    }

    fn skip(&mut self, n: u64) -> Result<(), AppError> {
        std::io::copy(&mut self.inner.by_ref().take(n), &mut std::io::sink())
            .map_err(|e| invalid(format!("truncated metadata: {e}")))?;
        Ok(())
    }

    /// Read a metadata value of the given GGUF type. Returns Some(u64) for
    /// integer values (so we can capture context_length), Some via `out_str`
    /// for strings, and None for everything else (skipped).
    fn value(&mut self, ty: u32, out_str: &mut Option<String>) -> Result<Option<u64>, AppError> {
        match ty {
            0 | 1 => {
                // u8 / i8
                self.skip(1)?;
                Ok(None)
            }
            2 | 3 => {
                self.skip(2)?;
                Ok(None)
            }
            4 => Ok(Some(self.u32()? as u64)),
            5 => {
                // i32
                self.skip(4)?;
                Ok(None)
            }
            6 => {
                // f32
                self.skip(4)?;
                Ok(None)
            }
            7 => {
                // bool
                self.skip(1)?;
                Ok(None)
            }
            8 => {
                *out_str = Some(self.string()?);
                Ok(None)
            }
            9 => {
                // array: elem type + count, then elements
                let elem_ty = self.u32()?;
                let count = self.u64()?;
                if count > 10_000_000 {
                    return Err(invalid(format!("metadata array too long: {count}")));
                }
                for _ in 0..count {
                    let mut ignored = None;
                    self.value(elem_ty, &mut ignored)?;
                }
                Ok(None)
            }
            10 => Ok(Some(self.u64()?)),
            11 => {
                // i64
                self.skip(8)?;
                Ok(None)
            }
            12 => {
                // f64
                self.skip(8)?;
                Ok(None)
            }
            other => Err(invalid(format!("unknown metadata value type {other}"))),
        }
    }
}

/// Validate the GGUF header of `path` and extract basic metadata.
pub fn inspect(path: &Path) -> Result<GgufInfo, AppError> {
    if !path.is_file() {
        return Err(AppError::new(
            ErrorCode::ModelFileMissing,
            Some(format!("not found: {}", path.display())),
        ));
    }

    let file = File::open(path)?;
    let mut r = Reader {
        inner: BufReader::new(file),
    };

    if r.u32()? != GGUF_MAGIC {
        return Err(invalid("missing GGUF magic bytes"));
    }
    let version = r.u32()?;
    if !(1..=3).contains(&version) {
        return Err(invalid(format!("unsupported GGUF version {version}")));
    }
    let _tensor_count = r.u64()?;
    let kv_count = r.u64()?;
    if kv_count > MAX_KV_COUNT {
        return Err(invalid(format!("implausible metadata count {kv_count}")));
    }

    let mut trained_context_length = None;
    let mut model_name = None;

    for _ in 0..kv_count {
        let key = r.string()?;
        let ty = r.u32()?;
        let mut str_val = None;
        let int_val = r.value(ty, &mut str_val)?;

        if key.ends_with(".context_length") {
            trained_context_length = int_val;
        } else if key == "general.name" {
            model_name = str_val;
        }
    }

    Ok(GgufInfo {
        version,
        trained_context_length,
        model_name,
    })
}

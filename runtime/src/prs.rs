//! Reconstruction of GSNE8P rev 0 main.dol 0x800d2684..0x800d2828.
//! Original executable SHA-256: a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e.
//! LSB-first control bits, little-endian long token, overlapping backreferences.
//! Added bounds/output checks intentionally reject malformed inputs instead of
//! reproducing the original routine's out-of-bounds memory accesses.

struct Reader<'a> { bytes: &'a [u8], pos: usize, control: u8, remaining: u8 }
impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.bytes.get(self.pos).ok_or("truncated PRS input")?;
        self.pos += 1;
        Ok(b)
    }
    fn bit(&mut self) -> Result<usize, String> {
        if self.remaining == 0 { self.control = self.byte()?; self.remaining = 8; }
        let b = self.control & 1;
        self.control >>= 1; self.remaining -= 1;
        Ok(b as usize)
    }
}

pub fn decompress(input: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let mut r = Reader { bytes: input, pos: 0, control: 0, remaining: 0 };
    let mut out = Vec::new();
    loop {
        if r.bit()? == 1 {
            if out.len() == limit { return Err("PRS output limit exceeded".into()); }
            out.push(r.byte()?);
        } else {
            let (distance, length) = if r.bit()? == 1 {
                let lo = r.byte()? as usize;
                let token = lo | ((r.byte()? as usize) << 8);
                if token == 0 { return Ok(out); }
                let len = if lo & 7 == 0 { r.byte()? as usize + 1 } else { (lo & 7) + 2 };
                (8192 - (token >> 3), len)
            } else {
                let len = ((r.bit()? << 1) | r.bit()?) + 2;
                (256 - r.byte()? as usize, len)
            };
            if distance > out.len() { return Err("PRS backreference before output".into()); }
            if length > limit.saturating_sub(out.len()) { return Err("PRS output limit exceeded".into()); }
            for _ in 0..length { out.push(out[out.len() - distance]); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn literal_and_end() { assert_eq!(decompress(&[0x17,b'A',b'B',b'C',0,0],3).unwrap(), b"ABC"); }
    #[test] fn empty_stream() { assert!(decompress(&[2,0,0],0).unwrap().is_empty()); }
    #[test] fn reject_truncation() { for b in [&[][..], &[1], &[2,0]] { assert!(decompress(b,100).is_err()); } }
    #[test] fn reject_bad_backreference() { assert!(decompress(&[0,255],100).is_err()); }
    #[test] fn bound_output() { assert!(decompress(&[0x17,b'A',b'B',b'C',0,0],2).is_err()); }
    #[test] fn overlapping_short_copy() {
        // literal A, short copy length 2 distance 1, end marker.
        assert_eq!(decompress(&[0x41,b'A',255,0,0],10).unwrap(), b"AAA");
    }
}

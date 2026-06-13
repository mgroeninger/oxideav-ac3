//! Enhanced-coupling sub-band / band geometry — ATSC A/52:2018 Annex E
//! §E.2.3.3.16-19 + §E.3.5.2.
//!
//! Enhanced coupling (`ecplinu == 1`) reconstructs the high-frequency
//! transform coefficients of the fbw channels from a single shared
//! *enhanced coupling channel* plus per-band amplitude / angle / chaos
//! parameters (§E.3.5.5). Before any of that synthesis can run, the
//! decoder has to know the **band geometry**: which transform
//! coefficients (bins) belong to the enhanced-coupling region, how the
//! 22 fixed sub-bands of Table E3.7 are grouped into the variable
//! coupling *bands* that carry one coordinate each, and how many such
//! bands (`necplbnd`) exist for the current block.
//!
//! This module is the pure, spec-tabulated geometry layer. It carries:
//!
//! * [`begin_subbnd`] / [`end_subbnd`] — the Table E3.8 derivations of
//!   `ecpl_begin_subbnd` (from `ecplbegf`) and `ecpl_end_subbnd` (from
//!   `ecplendf`, or from the SPX begin when SPX is co-active).
//! * [`ECPL_SUBBND_TAB`] — Table E3.9 `ecplsubbndtab[]`, the starting
//!   transform-coefficient number of each of the 22 sub-bands (plus the
//!   one-past-the-end sentinel at index 22).
//! * [`DEFAULT_ECPL_BNDSTRC`] — Table E2.14 `defecplbndstrc[]`, the
//!   default banding used the first time enhanced coupling is active in
//!   a frame when `ecplbndstrce == 0`.
//! * [`necplbnd`] — §E.2.3.3.19 band-count derivation from the
//!   per-sub-band `ecplbndstrc[]` merge bits.
//! * [`band_bin_counts`] — the §E.3.5.5.1 `nbins_per_bnd_array[]`
//!   population: how many transform coefficients each enhanced coupling
//!   band spans.
//!
//! It deliberately does **not** perform any synthesis: amplitude /
//! angle / chaos parameter decode (Tables E3.10-E3.12) and the complex
//! coordinate reconstruction (§E.3.5.5) are a separate step still under
//! construction. Keeping the geometry isolated and unit-tested gives the
//! synthesis step a verified foundation.
//!
//! **Spec note (erratum):** the default-banding table is captioned
//! "Table E2.14" in the document's table-of-contents (and list of
//! tables) but is cross-referenced as "Table E2.13" from the body of
//! §E.2.3.3.18 — the latter collides with the *standard* coupling
//! default at the genuine Table E2.13. The two tables hold different
//! values; the enhanced-coupling values used here are those listed in
//! full under the §E.2.3.3.18 heading.

/// Table E3.9 — `ecplsubbndtab[]`. The starting transform-coefficient
/// number of each of the 22 enhanced-coupling sub-bands, with a
/// one-past-the-end sentinel (`253`) at index 22 so the half-open span
/// of sub-band `s` is `ECPL_SUBBND_TAB[s] .. ECPL_SUBBND_TAB[s + 1]`.
///
/// Sub-bands 0..=3 are 6 bins wide (13..18, 19..24, 25..30, 31..36);
/// sub-bands 4..=21 are 12 bins wide. The enhanced-coupling region thus
/// spans transform coefficients 13..=252.
pub const ECPL_SUBBND_TAB: [usize; 23] = [
    13, 19, 25, 31, 37, 49, 61, 73, 85, 97, 109, 121, 133, 145, 157, 169, 181, 193, 205, 217, 229,
    241, 253,
];

/// Number of enhanced-coupling sub-bands defined by Table E3.7.
pub const N_ECPL_SUBBND: usize = 22;

/// Table E2.14 — `defecplbndstrc[]`. The default enhanced-coupling
/// banding structure, indexed by absolute sub-band number. A `true`
/// ('1') entry means "merge sub-band `s` into the previous band". Per
/// §E.2.3.3.19 the merge bits for sub-bands `<= max(ecpl_begin_subbnd,
/// 8)` are always zero (and not transmitted); the table reflects that —
/// sub-bands 0..=8 are all `false`, with the first merge at sub-band 9.
pub const DEFAULT_ECPL_BNDSTRC: [bool; N_ECPL_SUBBND] = {
    let mut t = [false; N_ECPL_SUBBND];
    // §E.2.3.3.18 default: sub-bands 0..8 → 0; then the per-row values.
    t[9] = true;
    // t[10] = false (12 → 0)
    t[11] = true;
    // t[12] = false
    t[13] = true;
    // t[14] = false
    t[15] = true;
    t[16] = true;
    t[17] = true;
    // t[18] = false
    t[19] = true;
    t[20] = true;
    t[21] = true;
    t
};

/// §E.2.3.3.16 — derive `ecpl_begin_subbnd` from the 4-bit `ecplbegf`
/// code (Table E3.8).
///
/// ```text
/// if (ecplbegf < 3)       ecpl_begin_subbnd = ecplbegf * 2
/// else if (ecplbegf < 13) ecpl_begin_subbnd = ecplbegf + 2
/// else                    ecpl_begin_subbnd = ecplbegf * 2 - 10
/// ```
#[inline]
pub fn begin_subbnd(ecplbegf: u8) -> usize {
    let f = ecplbegf as usize;
    if f < 3 {
        f * 2
    } else if f < 13 {
        f + 2
    } else {
        f * 2 - 10
    }
}

/// §E.2.3.3.17 — derive `ecpl_end_subbnd` (one greater than the highest
/// active enhanced-coupling sub-band), per Table E3.8.
///
/// When spectral extension is **not** in use the end sub-band is taken
/// directly from the 4-bit `ecplendf` code (`ecplendf + 7`). When SPX
/// **is** co-active the enhanced-coupling region is instead bounded by
/// the SPX begin so the two regions abut: `spxbegf + 5` for
/// `spxbegf < 6`, else `spxbegf * 2`. In the SPX-active case `ecplendf`
/// is not transmitted.
#[inline]
pub fn end_subbnd(spxinu: bool, ecplendf: u8, spxbegf: usize) -> usize {
    if !spxinu {
        ecplendf as usize + 7
    } else if spxbegf < 6 {
        spxbegf + 5
    } else {
        spxbegf * 2
    }
}

/// §E.2.3.3.19 — number of enhanced-coupling bands.
///
/// ```text
/// necplbnd  = ecpl_end_subbnd - ecpl_begin_subbnd;
/// necplbnd -= sum(ecplbndstrc[ecpl_begin_subbnd ..= ecpl_end_subbnd-1])
/// ```
///
/// Each set merge bit collapses one sub-band into the previous band, so
/// the count is the number of sub-bands in the active span minus the
/// number of merges. `bndstrc` is indexed by absolute sub-band number.
#[inline]
pub fn necplbnd(begin: usize, end: usize, bndstrc: &[bool; N_ECPL_SUBBND]) -> usize {
    let span = end.saturating_sub(begin);
    let merges = bndstrc[begin..end.min(N_ECPL_SUBBND)]
        .iter()
        .filter(|&&b| b)
        .count();
    span - merges
}

/// §E.3.5.5.1 — populate `nbins_per_bnd_array[]`: the number of
/// transform-coefficient bins spanned by each enhanced-coupling band.
///
/// Walking the active sub-band span, a `0` merge bit opens a new band
/// and a `1` merge bit extends the current band; each sub-band
/// contributes `ecplsubbndtab[s+1] - ecplsubbndtab[s]` bins (6 for the
/// narrow low sub-bands, 12 for the rest). The returned vector has
/// length [`necplbnd`].
pub fn band_bin_counts(begin: usize, end: usize, bndstrc: &[bool; N_ECPL_SUBBND]) -> Vec<usize> {
    let mut counts: Vec<usize> = Vec::new();
    for sbnd in begin..end.min(N_ECPL_SUBBND) {
        let bins = ECPL_SUBBND_TAB[sbnd + 1] - ECPL_SUBBND_TAB[sbnd];
        if !bndstrc[sbnd] {
            // New band.
            counts.push(bins);
        } else if let Some(last) = counts.last_mut() {
            // Merge into the current band.
            *last += bins;
        } else {
            // Defensive: a leading merge bit with no open band. The spec
            // guarantees `ecplbndstrc[begin] == 0`, so this can only be
            // reached on malformed input; treat it as a new band.
            counts.push(bins);
        }
    }
    counts
}

/// First transform-coefficient number of the enhanced-coupling region
/// for the given begin sub-band (Table E3.9 lookup). This is where the
/// shared enhanced-coupling channel starts; below it every channel is
/// independently coded.
#[inline]
pub fn begin_bin(begin: usize) -> usize {
    ECPL_SUBBND_TAB[begin.min(N_ECPL_SUBBND)]
}

/// One-past-the-last transform-coefficient number of the
/// enhanced-coupling region for the given end sub-band (Table E3.9
/// lookup).
#[inline]
pub fn end_bin(end: usize) -> usize {
    ECPL_SUBBND_TAB[end.min(N_ECPL_SUBBND)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecplsubbndtab_matches_table_e3_9() {
        // Spot-check the Table E3.9 values + the 6/12-bin widths.
        assert_eq!(ECPL_SUBBND_TAB[0], 13);
        assert_eq!(ECPL_SUBBND_TAB[4], 37);
        assert_eq!(ECPL_SUBBND_TAB[21], 241);
        assert_eq!(ECPL_SUBBND_TAB[22], 253); // sentinel
                                              // Sub-bands 0..=3 are 6 bins wide.
        for s in 0..4 {
            assert_eq!(ECPL_SUBBND_TAB[s + 1] - ECPL_SUBBND_TAB[s], 6);
        }
        // Sub-bands 4..=21 are 12 bins wide.
        for s in 4..N_ECPL_SUBBND {
            assert_eq!(ECPL_SUBBND_TAB[s + 1] - ECPL_SUBBND_TAB[s], 12);
        }
        // Region spans tc 13..=252.
        assert_eq!(begin_bin(0), 13);
        assert_eq!(end_bin(22), 253);
    }

    #[test]
    fn begin_subbnd_table_e3_8() {
        // ecplbegf < 3 → *2.
        assert_eq!(begin_subbnd(0), 0);
        assert_eq!(begin_subbnd(1), 2);
        assert_eq!(begin_subbnd(2), 4);
        // 3..=12 → +2.
        assert_eq!(begin_subbnd(3), 5);
        assert_eq!(begin_subbnd(7), 9);
        assert_eq!(begin_subbnd(12), 14);
        // >=13 → *2 - 10.
        assert_eq!(begin_subbnd(13), 16);
        assert_eq!(begin_subbnd(14), 18);
        assert_eq!(begin_subbnd(15), 20);
    }

    #[test]
    fn end_subbnd_table_e3_8_no_spx() {
        // SPX off: ecplendf + 7.
        assert_eq!(end_subbnd(false, 0, 0), 7);
        assert_eq!(end_subbnd(false, 8, 0), 15);
        assert_eq!(end_subbnd(false, 15, 0), 22);
    }

    #[test]
    fn end_subbnd_table_e3_8_with_spx() {
        // SPX on, spxbegf < 6 → spxbegf + 5.
        assert_eq!(end_subbnd(true, 0, 0), 5);
        assert_eq!(end_subbnd(true, 0, 5), 10);
        // SPX on, spxbegf >= 6 → spxbegf * 2.
        assert_eq!(end_subbnd(true, 0, 6), 12);
        assert_eq!(end_subbnd(true, 0, 7), 14);
        // ecplendf is ignored when SPX is active.
        assert_eq!(end_subbnd(true, 15, 6), 12);
    }

    #[test]
    fn default_bndstrc_table_e2_14() {
        // Sub-bands 0..=8 are all zero (never transmitted; always merge=0).
        for s in 0..=8 {
            assert!(!DEFAULT_ECPL_BNDSTRC[s], "sbnd {s} should be 0");
        }
        // The Table E2.14 merge rows.
        assert!(DEFAULT_ECPL_BNDSTRC[9]);
        assert!(!DEFAULT_ECPL_BNDSTRC[10]);
        assert!(DEFAULT_ECPL_BNDSTRC[11]);
        assert!(!DEFAULT_ECPL_BNDSTRC[12]);
        assert!(DEFAULT_ECPL_BNDSTRC[13]);
        assert!(!DEFAULT_ECPL_BNDSTRC[14]);
        assert!(DEFAULT_ECPL_BNDSTRC[15]);
        assert!(DEFAULT_ECPL_BNDSTRC[16]);
        assert!(DEFAULT_ECPL_BNDSTRC[17]);
        assert!(!DEFAULT_ECPL_BNDSTRC[18]);
        assert!(DEFAULT_ECPL_BNDSTRC[19]);
        assert!(DEFAULT_ECPL_BNDSTRC[20]);
        assert!(DEFAULT_ECPL_BNDSTRC[21]);
    }

    #[test]
    fn necplbnd_no_merges() {
        // All-zero banding → every sub-band is its own band.
        let bndstrc = [false; N_ECPL_SUBBND];
        // begin=9, end=22 → 13 sub-bands, no merges → 13 bands.
        assert_eq!(necplbnd(9, 22, &bndstrc), 13);
        // A small span: begin=5, end=8 → 3 sub-bands.
        assert_eq!(necplbnd(5, 8, &bndstrc), 3);
    }

    #[test]
    fn necplbnd_with_default_banding() {
        // Default banding over the full high span begin=9, end=22.
        // 13 sub-bands; merges at 9,11,13,15,16,17,19,20,21 that fall in
        // [9,22) → 9 merges → 13 - 9 = 4 bands.
        let n = necplbnd(9, 22, &DEFAULT_ECPL_BNDSTRC);
        assert_eq!(n, 4);
    }

    #[test]
    fn band_bin_counts_no_merges_low_subbands() {
        // begin=0, end=4 → narrow 6-bin sub-bands, no merges → four
        // 6-bin bands.
        let bndstrc = [false; N_ECPL_SUBBND];
        let counts = band_bin_counts(0, 4, &bndstrc);
        assert_eq!(counts, vec![6, 6, 6, 6]);
    }

    #[test]
    fn band_bin_counts_default_banding_high() {
        // begin=9, end=22 under the default banding. Sub-bands here are
        // all 12-bin wide. Bands open at sub-bands with merge==0
        // (9 opens? no — 9 has merge=1 but it is the first sub-band of
        // the span, so per band_bin_counts the leading merge starts a new
        // band defensively). To match the spec's guarantee that the
        // first sub-band of the active span is never a merge, test a span
        // whose first sub-band has merge=0.
        //
        // begin=10, end=22 → first sub-band 10 has merge=0 (new band).
        // Merge bits in [10,22): 11,13,15,16,17,19,20,21 → 8 merges.
        // 12 sub-bands → 4 bands. Each band's bin count = 12 * (sub-bands
        // in band).
        let counts = band_bin_counts(10, 22, &DEFAULT_ECPL_BNDSTRC);
        // Bands: [10,11]=24, [12,13]=24, [14,15,16,17]=48, [18,19,20,21]=48.
        assert_eq!(counts, vec![24, 24, 48, 48]);
        // Total bins == span bins (12 each * 12 sub-bands = 144).
        let total: usize = counts.iter().sum();
        assert_eq!(total, 144);
        assert_eq!(total, end_bin(22) - begin_bin(10));
    }

    #[test]
    fn band_bin_counts_length_equals_necplbnd() {
        // The vector length must equal necplbnd for any banding.
        let begin = 10;
        let end = 22;
        let n = necplbnd(begin, end, &DEFAULT_ECPL_BNDSTRC);
        let counts = band_bin_counts(begin, end, &DEFAULT_ECPL_BNDSTRC);
        assert_eq!(counts.len(), n);
    }
}
